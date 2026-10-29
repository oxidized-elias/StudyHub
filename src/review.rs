use super::*;

pub(super) fn review_schema(db: &Connection) -> AResult<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS human_reviews(session_id INTEGER PRIMARY KEY, payload TEXT NOT NULL, edits TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS human_lessons(lesson_id INTEGER PRIMARY KEY, before_body TEXT, after_body TEXT, changes TEXT, feedback TEXT);
        CREATE TABLE IF NOT EXISTS source_files(id INTEGER PRIMARY KEY AUTOINCREMENT, session_id INTEGER, filename TEXT, path TEXT);")?;
    Ok(())
}

#[derive(Serialize, Deserialize, Clone)]
pub(super) struct ReviewPayload {
    pub j: J,
    pub title: String,
    pub draft: Value,
    pub transcript: String,
    pub files: Vec<String>,
    pub existing: bool,
}
#[derive(Serialize, Deserialize, Clone, Default)]
pub(super) struct HumanEdit {
    pub title: String,
    pub body: String,
    pub summary: String,
    pub unit_name: String,
    #[serde(default)]
    pub feedback: String,
    #[serde(default)]
    pub action: String,
}
impl ReviewPayload {
    fn initial_edit(&self) -> HumanEdit {
        HumanEdit {
            title: self.title.clone(),
            body: self.draft["body"].as_str().unwrap_or("").into(),
            summary: self.draft["summary"].as_str().unwrap_or("").into(),
            unit_name: self.draft["unit_name"].as_str().unwrap_or("").into(),
            ..HumanEdit::default()
        }
    }
}

pub(super) fn stage_review(a: &App, sid: i64, p: &ReviewPayload) -> AResult<()> {
    let mut db = a.db.lock().unwrap();
    let tx = db.transaction()?;
    tx.execute(
        "INSERT INTO human_reviews(session_id,payload,edits) VALUES(?1,?2,?3)
        ON CONFLICT(session_id) DO UPDATE SET payload=excluded.payload,edits=excluded.edits",
        params![
            sid,
            serde_json::to_string(p)?,
            serde_json::to_string(&p.initial_edit())?
        ],
    )?;
    tx.execute(
        "UPDATE sessions SET status='review',error='' WHERE id=?1",
        [sid],
    )?;
    tx.commit()?;
    drop(db);
    log(
        a,
        sid,
        "human_review",
        json!({"actor":"Human review", "output":"Draft ready. Waiting for your edits and approval before saving."}),
    );
    Ok(())
}

fn load_review(a: &App, sid: i64) -> AResult<(ReviewPayload, HumanEdit)> {
    let db = a.db.lock().unwrap();
    let (p, e): (String, String) = db.query_row(
        "SELECT payload,edits FROM human_reviews WHERE session_id=?1",
        [sid],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    Ok((serde_json::from_str(&p)?, serde_json::from_str(&e)?))
}

fn source_mime(filename: &str) -> &'static str {
    match Path::new(filename)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase()
        .as_str()
    {
        "pdf" => "application/pdf",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        "md" | "markdown" | "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

pub(super) fn save_originals(a: &App, sid: i64, files: &[(String, Vec<u8>)]) -> AResult<()> {
    let root = a.cfg.parent().unwrap_or(Path::new("data"));
    let dir = root.join("review/sources").join(sid.to_string());
    std::fs::create_dir_all(&dir)?;
    let mut db = a.db.lock().unwrap();
    let tx = db.transaction()?;
    for (filename, bytes) in files {
        tx.execute(
            "INSERT INTO source_files(session_id,filename,path) VALUES(?1,?2,'')",
            params![sid, filename],
        )?;
        let id = tx.last_insert_rowid();
        let rel = format!("review/sources/{sid}/{id}-{}", safe(filename));
        std::fs::write(root.join(&rel), bytes)?;
        tx.execute(
            "UPDATE source_files SET path=?1 WHERE id=?2",
            params![rel, id],
        )?;
    }
    tx.commit()?;
    Ok(())
}

pub(super) fn original_files(a: &App, sid: i64) -> AResult<Vec<(String, Vec<u8>)>> {
    let db = a.db.lock().unwrap();
    let mut st =
        db.prepare("SELECT filename,path FROM source_files WHERE session_id=?1 ORDER BY id")?;
    let files = st
        .query_map([sid], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let root = a.cfg.parent().unwrap_or(Path::new("data"));
    files
        .into_iter()
        .map(|(n, p)| Ok((n, std::fs::read(root.join(p))?)))
        .collect()
}

pub(super) async fn original_source(
    State(a): State<Arc<App>>,
    AxPath((sid, source)): AxPath<(i64, i64)>,
) -> R<Response> {
    let (name, path): (String, String) = a.db.lock().unwrap().query_row(
        "SELECT filename,path FROM source_files WHERE id=?1 AND session_id=?2",
        params![source, sid],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let bytes = std::fs::read(a.cfg.parent().unwrap_or(Path::new("data")).join(path))?;
    Ok((
        [
            (axum::http::header::CONTENT_TYPE, source_mime(&name)),
            (axum::http::header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        ],
        bytes,
    )
        .into_response())
}

pub(super) fn source_descriptors(a: &App, sid: i64, files: &[String]) -> AResult<Vec<Value>> {
    let mut sources = {
        let db = a.db.lock().unwrap();
        let mut st =
            db.prepare("SELECT id,filename FROM source_files WHERE session_id=?1 ORDER BY id")?;
        let rows = st.query_map([sid], |r| { let id: i64=r.get(0)?; let name: String=r.get(1)?;
            Ok(json!({"name":name,"type":source_mime(&name),"url":format!("/api/review/session/{sid}/source/{id}")}))
        })?.collect::<Result<Vec<_>,_>>()?;
        rows
    };
    for (i, name) in files.iter().enumerate() {
        sources.push(
            json!({"name":format!("Saved source {}: {name}",i+1), "type":source_mime(name),
            "url":format!("/api/review/session/{sid}/pdf/{}", url_segment(name))}),
        );
    }
    Ok(sources)
}

pub(super) async fn human_review(
    State(a): State<Arc<App>>,
    AxPath(sid): AxPath<i64>,
) -> R<Json<Value>> {
    let (p, e) = load_review(&a, sid)?;
    let sources = source_descriptors(&a, sid, &p.files)?;
    Ok(Json(
        json!({"draft":p.initial_edit(),"edits":e,"sources":sources,"transcript":p.transcript,"locked_title":p.existing,"class_type":p.j.ct,"course":p.j.course}),
    ))
}

fn url_segment(s: &str) -> String {
    s.as_bytes()
        .iter()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-_.~".contains(b) {
                (*b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

pub(super) async fn save_human_review(
    State(a): State<Arc<App>>,
    AxPath(sid): AxPath<i64>,
    Json(e): Json<HumanEdit>,
) -> R<Json<Value>> {
    let count = a.db.lock().unwrap().execute(
        "UPDATE human_reviews SET edits=?1 WHERE session_id=?2",
        params![serde_json::to_string(&e)?, sid],
    )?;
    if count == 0 {
        return Err(anyhow!("This review is no longer pending").into());
    }
    Ok(Json(json!({"ok":true})))
}

// Bounded LCS diff, with an inexpensive fallback for very large revisions.
pub(super) fn changes(before: &str, after: &str) -> String {
    if before == after {
        return String::new();
    }
    let a: Vec<&str> = before.split('\n').collect();
    let b: Vec<&str> = after.split('\n').collect();
    let mut start = 0;
    while start < a.len().min(b.len()) && a[start] == b[start] {
        start += 1;
    }
    let mut end_a = a.len();
    let mut end_b = b.len();
    while end_a > start && end_b > start && a[end_a - 1] == b[end_b - 1] {
        end_a -= 1;
        end_b -= 1;
    }
    let a = &a[start..end_a];
    let b = &b[start..end_b];
    let mut out = format!(
        "--- AI draft\n+++ Human revision\n@@ starting at line {} @@\n",
        start + 1
    );
    if (a.len() + 1).saturating_mul(b.len() + 1) > 1_000_000 {
        for l in a {
            out.push_str(&format!("-{l}\n"));
        }
        for l in b {
            out.push_str(&format!("+{l}\n"));
        }
        return out;
    }
    let width = b.len() + 1;
    let mut dp = vec![0u32; (a.len() + 1) * width];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            dp[i * width + j] = if a[i] == b[j] {
                1 + dp[(i + 1) * width + j + 1]
            } else {
                dp[(i + 1) * width + j].max(dp[i * width + j + 1])
            };
        }
    }
    let (mut i, mut j) = (0, 0);
    while i < a.len() || j < b.len() {
        if i < a.len() && j < b.len() && a[i] == b[j] {
            out.push_str(&format!(" {}\n", a[i]));
            i += 1;
            j += 1;
        } else if i < a.len() && (j == b.len() || dp[(i + 1) * width + j] >= dp[i * width + j + 1])
        {
            out.push_str(&format!("-{}\n", a[i]));
            i += 1;
        } else {
            out.push_str(&format!("+{}\n", b[j]));
            j += 1;
        }
    }
    out
}

fn snapshot(e: &HumanEdit) -> String {
    format!(
        "# {}\nSummary: {}\nUnit name: {}\n\n{}",
        e.title, e.summary, e.unit_name, e.body
    )
}
fn fence(s: &str, lang: &str) -> String {
    let size = regex::Regex::new(r"`+")
        .unwrap()
        .find_iter(s)
        .map(|m| m.len() + 1)
        .max()
        .unwrap_or(3)
        .max(3);
    let fence = "`".repeat(size);
    format!("{fence}{lang}\n{s}\n{fence}")
}

fn store_human_lesson(a: &App, sid: i64, p: &ReviewPayload, e: &HumanEdit) -> AResult<()> {
    let before = snapshot(&p.initial_edit());
    let after = snapshot(e);
    let diff = changes(&before, &after);
    if diff.is_empty() && e.feedback.trim().is_empty() {
        return Ok(());
    }
    let directory = a.cfg.parent().unwrap_or(Path::new("data")).join("lessons");
    std::fs::create_dir_all(&directory)?;
    let user = format!(
        "Human feedback: {}\nChanges to AI work:\n{diff}",
        e.feedback
    );
    let mut db = a.db.lock().unwrap();
    let tx = db.transaction()?;
    tx.execute("INSERT INTO lessons(ts,session_id,class_type,course,stage,kind,issues,ai_view,moderator_note,user_input,confirmed)
        VALUES(?1,?2,?3,?4,'human_review','user_edit','Human corrected the draft',?5,'',?6,1)",
        params![ts() as i64,sid,p.j.ct,p.j.course,brief(&p.draft),user])?;
    let id = tx.last_insert_rowid();
    tx.execute("INSERT INTO human_lessons(lesson_id,before_body,after_body,changes,feedback) VALUES(?1,?2,?3,?4,?5)", params![id,before,after,diff,e.feedback])?;
    let markdown = format!("# Human review: {}\n\nSubject: {}\nCourse: {}\nSession: {sid}\n\n## Feedback\n{}\n\n## Changes to AI work\n{}\n\n## AI draft\n{}\n\n## Human revision\n{}\n", e.title,p.j.ct,p.j.course,e.feedback,fence(&diff,"diff"),fence(&before,"markdown"),fence(&after,"markdown"));
    std::fs::write(directory.join(format!("{id}.md")), markdown)?;
    tx.commit()?;
    Ok(())
}

pub(super) fn finish_unattended(
    a: &App,
    sid: i64,
    p: ReviewPayload,
    vault: &Path,
) -> AResult<Value> {
    let mut edit = p.initial_edit();
    edit.action = "approve".into();
    finish_review_mode(a, sid, p, edit, vault, false)
}

pub(super) fn finish_review(
    a: &App,
    sid: i64,
    p: ReviewPayload,
    e: HumanEdit,
    vault: &Path,
) -> AResult<Value> {
    finish_review_mode(a, sid, p, e, vault, true)
}

fn finish_review_mode(
    a: &App,
    sid: i64,
    mut p: ReviewPayload,
    mut e: HumanEdit,
    vault: &Path,
    human: bool,
) -> AResult<Value> {
    if e.action == "discard" {
        let mut db = a.db.lock().unwrap();
        let tx = db.transaction()?;
        tx.execute("DELETE FROM human_reviews WHERE session_id=?1", [sid])?;
        tx.execute(
            "UPDATE sessions SET status=?1,error='Draft discarded during human review' WHERE id=?2",
            params![if p.existing { "done" } else { "failed" }, sid],
        )?;
        tx.commit()?;
        return Ok(json!({"discarded":true}));
    }
    if e.action != "approve" {
        bail!("Choose Save reviewed note or Discard draft");
    }
    if e.body.trim().is_empty() {
        bail!("The reviewed note cannot be empty");
    }
    if p.existing {
        e.title = p.title.clone();
    }
    e.title = safe(&e.title);
    if e.title.is_empty() {
        bail!("The reviewed note needs a title");
    }
    if !p.existing && index(vault).iter().any(|n| n.title == e.title) {
        e.title = format!("{} {sid}", e.title);
    }
    p.draft["title"] = json!(e.title);
    p.draft["body"] = json!(e.body);
    p.draft["summary"] = json!(e.summary);
    p.draft["unit_name"] = json!(e.unit_name);
    // The human's content is authoritative: do not send it back through the author.
    let j = &p.j;
    let title = &e.title;
    write_note(vault, j, title, &p.files, &p.transcript, &p.draft)?;
    if p.existing {
        master_remove(a, &j.ct, title);
    }
    // Reflect edited headings rather than obsolete AI-plan headings in the master list.
    let headings = regex::Regex::new(r"(?m)^##[ \t]+(.+?)[ \t]*$").unwrap();
    let ideas: Vec<Value> = headings
        .captures_iter(&e.body)
        .map(|c| json!({"name":c[1].trim(),"how":e.summary}))
        .collect();
    master_add(a, &j.ct, &ideas, title);
    master_file(a, vault, &j.ct);
    let cdir = vault.join(&j.ct).join(&j.course);
    let cd = cdir.join("Course Data.md");
    let mut lines: Vec<String> = std::fs::read_to_string(&cd)
        .unwrap_or_default()
        .lines()
        .filter(|l| l.starts_with("- [["))
        .map(String::from)
        .collect();
    let key = format!("- [[{title}]]");
    lines.retain(|l| !l.starts_with(&key));
    let files = p
        .files
        .iter()
        .map(|f| format!("[[{f}]]"))
        .collect::<Vec<_>>()
        .join(", ");
    lines.push(format!(
        "{key} - {}{} | Files: {files}",
        if j.unit.is_empty() {
            String::new()
        } else {
            format!("Unit {}: ", j.unit)
        },
        j.lesson
    ));
    std::fs::write(cd,format!("---\nclass_type: {}\ncourse: {}\ncourse_link: {}\ncourse_id: {}\n---\n# Course Data: {}\n- **Link:** {}\n- **ID:** {}\n- **Class type:** {}\n\n## Notes\n{}\n",json!(j.ct),json!(j.course),json!(j.link),json!(j.id),j.course,j.link,j.id,j.ct,lines.join("\n")))?;
    // Capture the original AI version, not the just-edited version.
    let original = if human {
        load_review(a, sid)?.0
    } else {
        p.clone()
    };
    if human {
        store_human_lesson(a, sid, &original, &e)?;
    }
    let path = format!("{}/{}/Mind Map Notes/{title}.md", j.ct, j.course);
    {
        let mut db = a.db.lock().unwrap();
        let tx = db.transaction()?;
        tx.execute("UPDATE sessions SET title=?1,note_path=?2,pdf=?3,transcript=?4,status='done',error='' WHERE id=?5",params![title,path,p.files.join("\n"),p.transcript,sid])?;
        tx.execute("DELETE FROM human_reviews WHERE session_id=?1", [sid])?;
        tx.commit()?;
    }
    // A newly approved revision owns the file just written in the active vault.
    let approved_path = std::fs::canonicalize(
        vault.join(format!("{}/{}/Mind Map Notes/{title}.md", j.ct, j.course)),
    )?;
    a.db.lock().unwrap().execute("INSERT INTO note_locations(session_id,path) VALUES(?1,?2) ON CONFLICT(session_id) DO UPDATE SET path=excluded.path",params![sid,approved_path.to_string_lossy()])?;
    log(
        a,
        sid,
        if human {
            "human_approved"
        } else {
            "bulk_auto_saved"
        },
        json!({"actor":if human {"Human"}else{"System: bulk skip human review"}, "input":original.draft["body"],"output":e.body,"changes":changes(&snapshot(&original.initial_edit()),&snapshot(&e)), "guidance":e.feedback}),
    );
    Ok(json!({"title":title,"path":path}))
}

pub(super) async fn complete_human_review(
    State(a): State<Arc<App>>,
    AxPath(sid): AxPath<i64>,
    Json(e): Json<HumanEdit>,
) -> R<Json<Value>> {
    let _guard = a
        .busy
        .try_lock()
        .map_err(|_| anyhow!("Wait for the current processing task to finish"))?;
    let vault = a.vault().await;
    let (p, _) = load_review(&a, sid)?;
    Ok(Json(finish_review(&a, sid, p, e, &vault)?))
}

pub(super) fn load_pending_file(a: &App, sid: i64, name: &str) -> bool {
    load_review(a, sid)
        .map(|(p, _)| p.files.iter().any(|f| f == name))
        .unwrap_or(false)
}

pub(super) fn normalize_markdown(body: &str) -> String {
    let body = body
        .trim_start_matches('\u{feff}')
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    let re =
        regex::Regex::new(r"(?is)\n*```[ \t]*mermaid\b([^`]*(?:`[^`]+)*)```[ \t]*\n*").unwrap();
    re.replace_all(&body, |c: &regex::Captures| {
        let mut diagram = c[1].trim().to_string();
        if !diagram.contains('\n') {
            diagram = diagram.replace("\t\t", "\n");
            let header =
                regex::Regex::new(r"(?i)^((?:graph|flowchart)[ \t]+(?:TD|TB|BT|LR|RL))[ \t]+")
                    .unwrap();
            diagram = header.replace(&diagram, "$1\n").to_string();
        }
        format!(
            "\n\n```mermaid\n{}\n```\n\n",
            diagram.replace('\t', "  ").trim()
        )
    })
    .trim()
    .to_string()
}

pub(super) fn review_source_names(a: &App, sid: i64) -> AResult<Vec<String>> {
    Ok(load_review(a, sid)?.0.files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Fixture {
        root: PathBuf,
        app: Arc<App>,
    }
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "notes-review-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(root.join("data/review")).unwrap();
            let db = Connection::open(root.join("data/review/sessions.db")).unwrap();
            initialize_database(&db).unwrap();
            let settings = Settings {
                vault_path: root.join("vault").to_string_lossy().into(),
                ..Settings::default()
            };
            let app = Arc::new(App {
                settings: Mutex::new(settings),
                cfg: root.join("data/config.json"),
                busy: Arc::new(Mutex::new(())),
                pending: std::sync::Mutex::new(HashMap::new()),
                tasks: std::sync::Mutex::new(HashMap::new()),
                http: reqwest::Client::new(),
                db: std::sync::Mutex::new(db),
            });
            Self { root, app }
        }
        fn pending(&self) -> (i64, ReviewPayload) {
            let j = J {
                ct: "Math".into(),
                course: "PreCalculus".into(),
                unit: String::new(),
                lesson: "Polynomials".into(),
                link: String::new(),
                id: String::new(),
            };
            let sid = new_session(&self.app, &j);
            let p = ReviewPayload {
                j,
                title: "Polynomials".into(),
                draft: json!({"title":"Polynomials","summary":"Degree","unit_name":"Polynomials","body":"## Degree\nAI draft text.\n```mermaid\nflowchart LR\nA --> B\n```"}),
                transcript: "Original student notes".into(),
                files: vec![],
                existing: false,
            };
            stage_review(&self.app, sid, &p).unwrap();
            (sid, p)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    #[tokio::test]
    async fn manual_note_preserves_markdown_and_rejects_overwrites_and_empty_content() {
        let f = Fixture::new();
        let body = "# My notes\n\n| A | B |\n|---|---|\n| 1 | 2 |\n\n```mermaid\nsequenceDiagram\nAlice->>Bob: Hello\n```\n\n$x^2$\n";
        let draft = || NewNote {
            title: "My notes".into(),
            class_type: "Math".into(),
            course: "Algebra".into(),
            body: body.into(),
        };
        let Json(created) = create_note(State(f.app.clone()), Json(draft()))
            .await
            .ok()
            .unwrap();
        let id = created["id"].as_i64().unwrap();
        let Json(saved) = session(State(f.app.clone()), AxPath(id))
            .await
            .ok()
            .unwrap();
        assert!(saved["note"].as_str().unwrap().contains(body.trim_end()));
        assert_eq!(saved["status"], "done");
        assert_eq!(saved["unit"], "");
        assert!(create_note(State(f.app.clone()), Json(draft()))
            .await
            .is_err());
        let mut empty = draft();
        empty.title = "Empty".into();
        empty.body = "  ".into();
        assert!(create_note(State(f.app.clone()), Json(empty))
            .await
            .is_err());
        let count: i64 = f
            .app
            .db
            .lock()
            .unwrap()
            .query_row("SELECT count(*) FROM sessions", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
        let Json(saved) = session(State(f.app.clone()), AxPath(id))
            .await
            .ok()
            .unwrap();
        assert!(saved["note"].as_str().unwrap().contains(body.trim_end()));
    }

    #[tokio::test]
    async fn legacy_transcript_is_removed_from_file_but_sources_stay_available() {
        let f = Fixture::new();
        let (sid, _) = f.pending();
        let relative = "Math/PreCalculus/Mind Map Notes/Polynomials.md";
        let file = f.root.join("vault").join(relative);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        let raw = "---\ntitle: Polynomials\n---\n> [!info]- Session input (used to connect notes)\n> **Course:** PreCalculus\n>\n> **Transcribed notes**\n> Original student notes\n> Second transcript line\n\n## Degree\nHuman-approved text.\n\n> A legitimate quote\n";
        std::fs::write(&file, raw).unwrap();
        f.app.db.lock().unwrap().execute("UPDATE sessions SET status='done',title='Polynomials',note_path=?1,transcript='Original student notes' WHERE id=?2",params![relative,sid]).unwrap();
        save_originals(
            &f.app,
            sid,
            &[("Original.md".into(), b"# Original upload".to_vec())],
        )
        .unwrap();
        integration::clean_legacy_note_inputs(&f.app).await.unwrap();
        let cleaned = std::fs::read_to_string(&file).unwrap();
        assert!(cleaned.starts_with("---\ntitle: Polynomials\n---\n"));
        assert!(!cleaned.contains("Transcribed notes"));
        assert!(!cleaned.contains("Original student notes"));
        assert!(cleaned.contains("Human-approved text."));
        assert!(cleaned.contains("> A legitimate quote"));
        let detail = session(State(f.app.clone()), AxPath(sid)).await.unwrap().0;
        assert_eq!(detail["transcript"], "Original student notes");
        assert_eq!(
            original_files(&f.app, sid).unwrap()[0].1,
            b"# Original upload"
        );
        integration::clean_legacy_note_inputs(&f.app).await.unwrap();
        assert_eq!(std::fs::read_to_string(file).unwrap(), cleaned);
    }

    #[tokio::test]
    async fn imported_note_resolves_old_vault_and_survives_another_vault_change() {
        let f = Fixture::new();
        let (sid, _) = f.pending();
        let relative = "Math/PreCalculus/Mind Map Notes/Polynomials.md";
        let file = f.root.join("vault").join(relative);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, "## Degree\nHuman-approved text").unwrap();
        f.app
            .db
            .lock()
            .unwrap()
            .execute(
                "UPDATE sessions SET status='done',title='Polynomials',note_path=?1 WHERE id=?2",
                params![relative, sid],
            )
            .unwrap();
        f.app.settings.lock().await.vault_path = f.root.join("new-vault").to_string_lossy().into();
        let exported = super::super::integration::note(
            State(f.app.clone()),
            AxPath(sid),
            axum::http::HeaderMap::new(),
        )
        .await
        .unwrap()
        .0;
        assert!(exported["body"]
            .as_str()
            .unwrap()
            .contains("Human-approved text"));
        // Recorded locations work even outside the original repository vault.
        let moved = f.root.join("other-vault/note.md");
        std::fs::create_dir_all(moved.parent().unwrap()).unwrap();
        std::fs::rename(&file, &moved).unwrap();
        f.app
            .db
            .lock()
            .unwrap()
            .execute(
                "UPDATE note_locations SET path=?1 WHERE session_id=?2",
                params![moved.to_string_lossy(), sid],
            )
            .unwrap();
        assert_eq!(
            super::super::integration::resolve_note_path(&f.app, sid, relative)
                .await
                .unwrap(),
            moved
        );
        std::fs::remove_file(&moved).unwrap();
        assert!(
            super::super::integration::resolve_note_path(&f.app, sid, relative)
                .await
                .unwrap_err()
                .to_string()
                .contains("Changing the vault setting does not move existing notes")
        );
    }

    #[tokio::test]
    async fn note_style_override_is_persistent_and_default_can_be_restored() {
        let f = Fixture::new();
        let (sid, _) = f.pending();
        super::super::note_preferences::save(&f.app, sid, "hybrid").unwrap();
        f.app.settings.lock().await.note_style = "text_chunks".into();
        let value = super::super::note_preferences::read(State(f.app.clone()), AxPath(sid))
            .await
            .unwrap()
            .0;
        assert_eq!(value["effective_style"], "hybrid");
        let db = Connection::open(f.root.join("data/review/sessions.db")).unwrap();
        assert_eq!(
            db.query_row(
                "SELECT note_style FROM note_preferences WHERE session_id=?1",
                [sid],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            "hybrid"
        );
        assert!(super::super::note_preferences::save(&f.app, sid, "invalid").is_err());
        assert!(super::super::note_preferences::save(&f.app, sid + 100, "hybrid").is_err());
        super::super::note_preferences::save(&f.app, sid, "").unwrap();
        assert_eq!(
            super::super::note_preferences::read(State(f.app.clone()), AxPath(sid))
                .await
                .unwrap()
                .0["effective_style"],
            "text_chunks"
        );
    }

    #[tokio::test]
    async fn review_is_durable_and_requires_explicit_approval() {
        let f = Fixture::new();
        let (sid, p) = f.pending();
        assert!(!f
            .root
            .join("vault/Math/PreCalculus/Mind Map Notes/Polynomials.md")
            .exists());
        let result = complete_human_review(
            State(f.app.clone()),
            AxPath(sid),
            Json(HumanEdit {
                action: "skip".into(),
                ..p.initial_edit()
            }),
        )
        .await;
        assert!(result.is_err());
        assert_eq!(load_row(&f.app, sid).unwrap().4, "review");
        let edit = HumanEdit {
            body: "## Degree\nMy corrected notes.".into(),
            feedback: "Keep my wording for definitions.".into(),
            ..p.initial_edit()
        };
        assert!(
            save_human_review(State(f.app.clone()), AxPath(sid), Json(edit.clone()))
                .await
                .is_ok()
        );
        // Reopen the database just as after a process restart: review and edits survive.
        let reopened = Connection::open(f.root.join("data/review/sessions.db")).unwrap();
        initialize_database(&reopened).unwrap();
        assert_eq!(recover_interrupted(&reopened).unwrap(), 0);
        let saved: String = reopened
            .query_row(
                "SELECT edits FROM human_reviews WHERE session_id=?1",
                [sid],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            serde_json::from_str::<HumanEdit>(&saved).unwrap().body,
            edit.body
        );
        let result = complete_human_review(
            State(f.app.clone()),
            AxPath(sid),
            Json(HumanEdit {
                action: "approve".into(),
                ..edit
            }),
        )
        .await;
        assert!(result.is_ok());
        let note = std::fs::read_to_string(
            f.root
                .join("vault/Math/PreCalculus/Mind Map Notes/Polynomials.md"),
        )
        .unwrap();
        assert!(note.contains("My corrected notes."));
        assert!(!note.contains("AI draft text."));
        assert_eq!(load_row(&f.app, sid).unwrap().4, "done");
        assert!(load_review(&f.app, sid).is_err());
        let rows = lessons_list(State(f.app.clone())).await.unwrap().0;
        assert_eq!(rows[0]["kind"], "user_edit");
        assert_eq!(rows[0]["confirmed"], true);
        assert!(rows[0]["changes"]
            .as_str()
            .unwrap()
            .contains("+My corrected notes."));
        assert!(rows[0]["before"]
            .as_str()
            .unwrap()
            .contains("AI draft text."));
        assert!(rows[0]["after"]
            .as_str()
            .unwrap()
            .contains("My corrected notes."));
        let id = rows[0]["id"].as_i64().unwrap();
        let lesson = std::fs::read_to_string(f.root.join(format!("data/lessons/{id}.md"))).unwrap();
        assert!(lesson.contains("Keep my wording"));
        assert!(lesson.contains("## Changes to AI work"));
        let settings = f.app.settings.lock().await.clone();
        let models = settings.models["ollama"].clone();
        let reader = Reader {
            vault: f.root.join("vault"),
            notes: vec![],
        };
        let c = Ctx {
            a: &f.app,
            s: &settings,
            sid,
            m: &models,
            list: "",
            titles: &[],
            input: "",
            tx: "Polynomials degree",
            gtxt: "",
            ct: "Math",
            course: "PreCalculus",
            reader: &reader,
        };
        let learned = lessons_for(&c).unwrap();
        assert!(learned.contains("Keep my wording"));
        assert!(learned.contains("+My corrected notes."));
        clear_note_logs(&mut f.app.db.lock().unwrap(), sid).unwrap();
        assert_eq!(
            lessons_list(State(f.app.clone()))
                .await
                .unwrap()
                .0
                .as_array()
                .unwrap()
                .len(),
            1
        );
        let _ = lesson_delete(State(f.app.clone()), AxPath(id)).await;
        assert!(!f.root.join(format!("data/lessons/{id}.md")).exists());
    }

    #[tokio::test]
    async fn original_file_bytes_are_preserved_and_scoped_to_session() {
        let f = Fixture::new();
        let (sid, _) = f.pending();
        let bytes = b"# Original\r\n$ x^2 $\r\n".to_vec();
        save_originals(&f.app, sid, &[("Original.md".into(), bytes.clone())]).unwrap();
        assert_eq!(original_files(&f.app, sid).unwrap()[0].1, bytes);
        let review = human_review(State(f.app.clone()), AxPath(sid))
            .await
            .unwrap()
            .0;
        assert_eq!(review["sources"][0]["name"], "Original.md");
        let source: i64 = f
            .app
            .db
            .lock()
            .unwrap()
            .query_row(
                "SELECT id FROM source_files WHERE session_id=?1",
                [sid],
                |r| r.get(0),
            )
            .unwrap();
        let response = original_source(State(f.app.clone()), AxPath((sid, source)))
            .await
            .unwrap();
        let data = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        assert_eq!(data.as_ref(), bytes);
        assert!(
            original_source(State(f.app.clone()), AxPath((sid + 1, source)))
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn discard_never_overwrites_an_existing_note_or_creates_a_lesson() {
        let f = Fixture::new();
        let (sid, mut p) = f.pending();
        p.existing = true;
        write_note(
            &f.root.join("vault"),
            &p.j,
            &p.title,
            &[],
            "Original",
            &json!({"body":"Existing note"}),
        )
        .unwrap();
        setcol(&f.app, sid, "title", &p.title);
        stage_review(&f.app, sid, &p).unwrap();
        let result = complete_human_review(
            State(f.app.clone()),
            AxPath(sid),
            Json(HumanEdit {
                action: "discard".into(),
                ..p.initial_edit()
            }),
        )
        .await;
        assert!(result.is_ok());
        assert_eq!(load_row(&f.app, sid).unwrap().4, "done");
        assert!(std::fs::read_to_string(
            f.root
                .join("vault/Math/PreCalculus/Mind Map Notes/Polynomials.md")
        )
        .unwrap()
        .contains("Existing note"));
        assert_eq!(
            lessons_list(State(f.app.clone()))
                .await
                .unwrap()
                .0
                .as_array()
                .unwrap()
                .len(),
            0
        );
    }

    #[test]
    fn line_diff_handles_insertions_deletions_and_large_revisions() {
        assert!(changes("same", "same").is_empty());
        let diff = changes("Heading\nWrong\nTail", "Heading\nCorrect\nExtra\nTail");
        assert!(diff.contains("-Wrong\n+Correct\n+Extra"));
        assert!(!diff.contains("-Heading"));
        let large = changes(&"old\n".repeat(1100), &"new\n".repeat(1100));
        assert!(large.contains("-old"));
        assert!(large.contains("+new"));
    }

    #[test]
    fn malformed_ai_mermaid_fences_are_repaired_without_damaging_mindmap_indent() {
        let raw = "## Forms\r\n```mermaid\r\ngraph TD\t\tA[Form] --> B[Degree];\t\tB --> C;\t\t```";
        let fixed = normalize_markdown(raw);
        assert!(fixed.contains("graph TD\nA[Form]"));
        assert!(fixed.ends_with("\n```"));
        assert!(!fixed.contains('\r'));
        let mindmap = "```mermaid\nmindmap\n  root((Notes))\n    Algebra\n      Degree\n```";
        assert_eq!(normalize_markdown(mindmap), mindmap);
        assert_eq!(normalize_markdown(&fixed), fixed);
    }
    #[tokio::test]
    async fn unattended_bulk_saves_without_human_records_or_waiting() {
        let f = Fixture::new();
        let (sid, p) = f.pending();
        f.app
            .db
            .lock()
            .unwrap()
            .execute("DELETE FROM human_reviews WHERE session_id=?1", [sid])
            .unwrap();
        f.app
            .db
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO import_options(session_id,skip_human)VALUES(?1,1)",
                [sid],
            )
            .unwrap();
        let response = tokio::time::timeout(
            Duration::from_millis(100),
            ask(&f.app, sid, "Unreadable symbol", json!([]), json!([]), None),
        )
        .await
        .unwrap();
        assert_eq!(response, ("skip".into(), String::new()));
        assert!(f.app.pending.lock().unwrap().is_empty());
        let vault = f.app.vault().await;
        let result = finish_unattended(&f.app, sid, p, &vault).unwrap();
        assert!(vault.join(result["path"].as_str().unwrap()).exists());
        let db = f.app.db.lock().unwrap();
        assert_eq!(
            db.query_row("SELECT status FROM sessions WHERE id=?1", [sid], |r| r
                .get::<_, String>(
                0
            ))
            .unwrap(),
            "done"
        );
        assert_eq!(
            db.query_row("SELECT count(*) FROM human_reviews", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            db.query_row("SELECT count(*) FROM human_lessons", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM events WHERE session_id=?1 AND kind='bulk_auto_saved'",
                [sid],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
    }
}
