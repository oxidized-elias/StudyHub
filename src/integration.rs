use super::*;
use anyhow::ensure;
use axum::http::HeaderMap;
use rusqlite::OptionalExtension;
use std::hash::{Hash, Hasher};
fn authorize(headers: &HeaderMap) -> AResult<()> {
    let token = std::env::var("NOTES_INTEGRATION_TOKEN").unwrap_or_default();
    if !token.is_empty() {
        let expected = format!("Bearer {token}");
        if headers.get("authorization").and_then(|h| h.to_str().ok()) != Some(expected.as_str()) {
            bail!("NoteSorter integration token is required");
        }
    }
    Ok(())
}
pub(super) async fn list(State(a): State<Arc<App>>, headers: HeaderMap) -> R<Json<Value>> {
    authorize(&headers)?;
    let db = a.db.lock().unwrap();
    let mut st=db.prepare("SELECT id,title,class_type,course,unit,lesson,note_path FROM sessions WHERE status IN ('done','review','running','waiting') AND title<>'' AND note_path<>'' ORDER BY class_type,course,id")?;
    let rows=st.query_map([],|r|Ok(json!({"id":r.get::<_,i64>(0)?,"title":r.get::<_,String>(1)?,"class_type":r.get::<_,String>(2)?,"course":r.get::<_,String>(3)?,"unit":r.get::<_,String>(4)?,"lesson":r.get::<_,String>(5)?,"note_path":r.get::<_,String>(6)?})))?.collect::<Result<Vec<_>,_>>()?;
    Ok(Json(json!(rows)))
}
// Remove only the legacy application-generated callout, not ordinary quotations.
pub(super) fn without_session_input(raw: &str) -> String {
    let mut skipping = false;
    raw.split_inclusive('\n')
        .filter(|line| {
            let text = line.trim_end_matches(['\r', '\n']);
            if text == "> [!info]- Session input (used to connect notes)" {
                skipping = true;
                return false;
            }
            if skipping && text.starts_with('>') {
                return false;
            }
            skipping = false;
            true
        })
        .collect()
}

pub(super) async fn clean_legacy_note_inputs(a: &Arc<App>) -> AResult<()> {
    let notes = {
        let db = a.db.lock().unwrap();
        let mut statement = db.prepare("SELECT id,note_path FROM sessions WHERE note_path<>''")?;
        let notes = statement
            .query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        notes
    };
    for (id, relative) in notes {
        let Ok(path) = resolve_note_path(a, id, &relative).await else {
            continue;
        };
        let Ok(raw) = std::fs::read_to_string(&path) else {
            continue;
        };
        let cleaned = without_session_input(&raw);
        if cleaned != raw {
            std::fs::write(path, cleaned)?;
        }
    }
    Ok(())
}

pub(super) fn approved_body(raw: &str) -> String {
    let raw = big_ideas::without_links(raw).replace("\r\n", "\n");
    let body = if raw.starts_with("---\n") {
        raw.splitn(3, "---\n").nth(2).unwrap_or(&raw)
    } else {
        &raw
    };
    let mut skipping = false;
    body.lines()
        .filter(|line| {
            if line.starts_with("> [!info]- Session input") {
                skipping = true;
                return false;
            }
            if skipping && line.starts_with('>') {
                return false;
            }
            if skipping && line.trim().is_empty() {
                skipping = false;
                return false;
            }
            skipping = false;
            true
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}
// Keep approved notes readable when Settings points future writes at another vault.
// Resolve only recorded or known vault locations; never recursively search the host.
pub(super) async fn resolve_note_path(a: &App, id: i64, relative: &str) -> AResult<PathBuf> {
    ensure!(
        !relative.is_empty(),
        "This note has no approved file yet; approve its review first"
    );
    let recorded: Option<String> =
        a.db.lock()
            .unwrap()
            .query_row(
                "SELECT path FROM note_locations WHERE session_id=?1",
                [id],
                |r| r.get(0),
            )
            .optional()?;
    let current = a.vault().await.join(relative);
    let legacy = a
        .cfg
        .parent()
        .and_then(Path::parent)
        .unwrap_or(Path::new("."))
        .join("vault")
        .join(relative);
    let mut paths = Vec::new();
    if let Some(path) = recorded {
        paths.push(PathBuf::from(path));
    }
    paths.push(current.clone());
    paths.push(legacy);
    for path in paths {
        if path.is_file() {
            let absolute = std::fs::canonicalize(&path)?;
            a.db.lock().unwrap().execute("INSERT INTO note_locations(session_id,path) VALUES(?1,?2) ON CONFLICT(session_id) DO UPDATE SET path=excluded.path",params![id,absolute.to_string_lossy()])?;
            return Ok(absolute);
        }
    }
    bail!("Approved note file is missing: {}. Changing the vault setting does not move existing notes. Restore the file or select its original vault in Settings.",current.display())
}
pub(super) async fn remember_locations(a: &App) -> AResult<()> {
    let rows = {
        let db = a.db.lock().unwrap();
        let mut st = db.prepare("SELECT id,note_path FROM sessions WHERE note_path<>''")?;
        let rows = st
            .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        rows
    };
    for (id, path) in rows {
        let _ = resolve_note_path(a, id, &path).await;
    }
    Ok(())
}
pub(super) async fn note(
    State(a): State<Arc<App>>,
    AxPath(id): AxPath<i64>,
    headers: HeaderMap,
) -> R<Json<Value>> {
    authorize(&headers)?;
    let (j, title, pdfs, transcript, status) = load_row(&a, id)?;
    if !["done", "review", "running", "waiting"].contains(&status.as_str()) || title.is_empty() {
        return Err(anyhow!("Only completed, human-approved notes can be used for study").into());
    }
    let path: String = a.db.lock().unwrap().query_row(
        "SELECT note_path FROM sessions WHERE id=?1",
        [id],
        |r| r.get(0),
    )?;
    let raw = std::fs::read_to_string(resolve_note_path(&a, id, &path).await?)?;
    let body = approved_body(&raw);
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    body.hash(&mut hash);
    let version = format!("{:016x}", hash.finish());
    let headings = body
        .lines()
        .filter_map(|l| l.strip_prefix("## "))
        .collect::<Vec<_>>();
    Ok(Json(
        json!({"id":id,"title":title,"class_type":j.ct,"course":j.course,"unit":j.unit,"lesson":j.lesson,"body":body,"headings":headings,"version":version,"transcript":transcript,"sources":review::source_descriptors(&a,id,&pdfs.lines().map(str::to_owned).collect::<Vec<_>>())?,"note_style":note_preferences::get(&a,id)}),
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn study_uses_approved_body_not_the_old_transcript_callout() {
        let raw="---\ntitle: Test\n---\n> [!info]- Session input\n> Old wording\n>\n> transcript\n\n## Corrected\nHuman-approved wording\n\n> A legitimate quote";
        let body = approved_body(raw);
        assert!(!body.contains("Old wording"));
        assert!(body.contains("Human-approved wording"));
        assert!(body.contains("> A legitimate quote"));
    }
}

pub(super) async fn config_get(State(a): State<Arc<App>>, headers: HeaderMap) -> R<Json<Value>> {
    authorize(&headers)?;
    let s = a.settings.lock().await;
    Ok(Json(
        json!({"vault_path":s.vault_path,"note_style":s.note_style,"provider":s.provider,"ollama_url":s.ollama_url,"models":s.models,"ai1_ctx":s.ai1_ctx}),
    ))
}
pub(super) async fn config_put(
    State(a): State<Arc<App>>,
    headers: HeaderMap,
    Json(v): Json<Value>,
) -> R<Json<Value>> {
    authorize(&headers)?;
    let mut s = a.settings.lock().await.clone();
    if let Some(path) = v["vault_path"].as_str() {
        s.vault_path = path.trim().into();
    }
    if let Some(style) = v["note_style"].as_str() {
        s.note_style = style.into();
    }
    if let Some(provider) = v["provider"].as_str() {
        if !["ollama", "openai", "claude", "gemini", "deepseek"].contains(&provider) {
            return Err(anyhow!("Unsupported notes provider").into());
        }
        s.provider = provider.into();
    }
    if let Some(url) = v["ollama_url"].as_str() {
        let parsed = reqwest::Url::parse(url.trim())?;
        if !["http", "https"].contains(&parsed.scheme()) || parsed.host_str().is_none() {
            return Err(anyhow!("Ollama URL must be an HTTP or HTTPS server address").into());
        }
        s.ollama_url = url.trim().into();
    }
    if let Some(models) = v.get("models") {
        s.models = serde_json::from_value(models.clone())?;
    }
    if let Some(key) = v["api_key"].as_str().filter(|key| !key.trim().is_empty()) {
        s.keys.insert(s.provider.clone(), key.trim().into());
    }
    let _ = put_settings(State(a.clone()), Json(s)).await?;
    config_get(State(a), headers).await
}

#[derive(Deserialize)]
pub(super) struct FolderQuery {
    pub path: Option<String>,
}
pub(super) async fn folders(
    State(a): State<Arc<App>>,
    headers: HeaderMap,
    Query(q): Query<FolderQuery>,
) -> R<Json<Value>> {
    authorize(&headers)?;
    let requested = q.path.unwrap_or_else(|| {
        a.cfg
            .parent()
            .unwrap_or(Path::new("."))
            .to_string_lossy()
            .into_owned()
    });
    let path = std::fs::canonicalize(vpath(&requested))?;
    let mut entries = std::fs::read_dir(&path)?
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .map(|e| json!({"name":e.file_name().to_string_lossy(),"path":e.path().to_string_lossy()}))
        .collect::<Vec<_>>();
    entries.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
    Ok(Json(
        json!({"path":path.to_string_lossy(),"parent":path.parent().map(|p|p.to_string_lossy().into_owned()),"folders":entries}),
    ))
}
