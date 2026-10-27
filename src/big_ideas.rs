//! Subject-wide, source-backed concept pages. Only generated blocks/pages are replaced.
use super::*;
use std::collections::BTreeMap;
const START: &str = "<!-- studyhub:big-ideas:start -->";
const END: &str = "<!-- studyhub:big-ideas:end -->";
const GENERATED: &str = "studyhub_generated: big_idea";
#[derive(Clone)]
struct Contribution {
    path: String,
    title: String,
    course: String,
    heading: String,
    body: String,
}
#[derive(Default)]
struct Idea {
    name: String,
    sources: Vec<Contribution>,
}
fn key(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}
fn filename(name: &str) -> String {
    let normalized = key(name);
    let stem = safe(&normalized);
    if stem == normalized && !stem.is_empty() {
        return format!("{stem}.md");
    }
    // Distinguish filesystem-sanitization collisions, e.g. x^2 and x2.
    let hash = normalized.bytes().fold(14695981039346656037u64, |h, b| {
        (h ^ b as u64).wrapping_mul(1099511628211)
    });
    format!(
        "{}-{hash:016x}.md",
        if stem.is_empty() { "idea" } else { &stem }
    )
}
fn wiki(path: &str, heading: &str, label: &str) -> String {
    format!(
        "[[{}{}|{}]]",
        path.trim_end_matches(".md"),
        if heading.is_empty() {
            String::new()
        } else {
            format!("#{heading}")
        },
        label
            .replace('|', " / ")
            .replace('[', "(")
            .replace(']', ")")
    )
}
pub(super) fn without_links(raw: &str) -> String {
    let mut result = raw.to_string();
    while let Some(start) = result.find(START) {
        let Some(end) = result[start..].find(END) else {
            break;
        };
        result.replace_range(start..start + end + END.len(), "");
    }
    result.trim_end().to_string()
}
fn sections(raw: &str) -> Vec<(String, String)> {
    let text = integration::approved_body(&without_links(raw));
    let mut result = Vec::new();
    let mut current: Option<(String, Vec<&str>)> = None;
    let mut fence: Option<(char, usize)> = None;
    for line in text.lines() {
        let trimmed = line.trim_start();
        let marker = trimmed.chars().next().unwrap_or(' ');
        let run = trimmed.chars().take_while(|c| *c == marker).count();
        if ['`', '~'].contains(&marker) && run >= 3 {
            match fence {
                None => fence = Some((marker, run)),
                Some((c, n)) if c == marker && run >= n => fence = None,
                _ => {}
            }
            if let Some((_, body)) = &mut current {
                body.push(line);
            }
            continue;
        }
        if fence.is_none() && (trimmed.starts_with("## ") || trimmed.starts_with("# ")) {
            if let Some((heading, body)) = current.take() {
                result.push((heading, body.join("\n").trim().to_string()));
            }
            if let Some(heading) = trimmed.strip_prefix("## ") {
                let heading = heading.trim().trim_end_matches('#').trim();
                if !heading.is_empty()
                    && !["topic index", "big ideas", "subject ideas"]
                        .contains(&key(heading).as_str())
                {
                    current = Some((heading.into(), Vec::new()));
                }
            }
        } else if let Some((_, body)) = &mut current {
            body.push(line);
        }
    }
    if let Some((heading, body)) = current {
        result.push((heading, body.join("\n").trim().to_string()));
    }
    result
}
fn write_changed(path: &Path, body: &str) -> AResult<()> {
    if std::fs::read_to_string(path).ok().as_deref() != Some(body) {
        std::fs::write(path, body)?;
    }
    Ok(())
}
pub(super) fn rebuild(vault: &Path) -> AResult<()> {
    std::fs::create_dir_all(vault)?;
    let mut subjects = dirs(vault);
    subjects.sort();
    let mut root =
        String::from("# Big ideas\n\nShared concepts across courses, grouped by subject.\n\n");
    for subject in subjects {
        let subject_name = name(&subject);
        let mut ideas: BTreeMap<String, Idea> = BTreeMap::new();
        let mut notes: Vec<(PathBuf, String, Vec<String>)> = Vec::new();
        let mut courses = dirs(&subject);
        courses.sort();
        for course in courses {
            let mm = course.join("Mind Map Notes");
            if !mm.is_dir() {
                continue;
            }
            let mut files = std::fs::read_dir(&mm)?
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("md") && !p.is_symlink())
                .collect::<Vec<_>>();
            files.sort();
            let mut course_index = format!(
                "# Notes: {}\n\n{}\n\n",
                name(&course),
                wiki(
                    &format!("{subject_name}/BigIdeas"),
                    "",
                    &format!("All {} big ideas", subject_name)
                )
            );
            for path in files {
                let raw = std::fs::read_to_string(&path)?;
                let title = path.file_stem().unwrap().to_string_lossy().to_string();
                let relative = path
                    .strip_prefix(vault)?
                    .to_string_lossy()
                    .replace('\\', "/");
                // Compatibility for existing course index links.
                let target = format!("Mind Map Notes/{title}.md")
                    .replace(' ', "%20")
                    .replace('(', "%28")
                    .replace(')', "%29")
                    .replace('#', "%23");
                course_index.push_str(&format!(
                    "- [{}](<{target}>)\n",
                    title.replace('[', "\\[").replace(']', "\\]")
                ));
                let mut keys = Vec::new();
                for (heading, body) in sections(&raw) {
                    let normalized = key(&heading);
                    if !keys.contains(&normalized) {
                        keys.push(normalized.clone())
                    }
                    let idea = ideas.entry(normalized).or_default();
                    if idea.name.is_empty() {
                        idea.name = heading.clone()
                    }
                    idea.sources.push(Contribution {
                        path: relative.clone(),
                        title: title.clone(),
                        course: name(&course),
                        heading,
                        body,
                    });
                }
                notes.push((path, raw, keys));
            }
            write_changed(&course.join("BigIdeas.md"), &course_index)?;
        }
        if notes.is_empty() && !subject.join("BigIdeas.md").exists() {
            continue;
        }
        let directory = subject.join("Big Ideas");
        std::fs::create_dir_all(&directory)?;
        let mut index=format!("# Big ideas: {subject_name}\n\nShared ideas from all courses in this subject. Each idea links to its contributing note sections.\n\n");
        let mut expected = std::collections::HashSet::new();
        for (normalized, idea) in &ideas {
            let file = filename(normalized);
            expected.insert(file.clone());
            let target = format!("{subject_name}/Big Ideas/{file}");
            index.push_str(&format!(
                "- {} ({} source sections)\n",
                wiki(&target, "", &idea.name),
                idea.sources.len()
            ));
            let mut page=format!("---\n{GENERATED}\n---\n# {}\n\n{}\n\nCollected from approved notes. Contributions retain their source; no AI synthesis is added.\n\n",idea.name,wiki(&format!("{subject_name}/BigIdeas"),"",&format!("All {subject_name} big ideas")));
            for source in &idea.sources {
                page.push_str(&format!(
                    "## {} — {}\n\nSource: {}\n\n{}\n\n",
                    source.title,
                    source.course,
                    wiki(&source.path, &source.heading, &source.heading),
                    source.body
                ));
            }
            write_changed(&directory.join(file), &page)?;
        }
        // Remove stale generated pages only; hand-written files are never deleted.
        for entry in std::fs::read_dir(&directory)?.filter_map(Result::ok) {
            if !expected.contains(&entry.file_name().to_string_lossy().to_string())
                && !entry.path().is_symlink()
                && std::fs::read_to_string(entry.path())
                    .unwrap_or_default()
                    .starts_with(&format!("---\n{GENERATED}\n"))
            {
                std::fs::remove_file(entry.path())?;
            }
        }
        for (path, raw, keys) in notes {
            let mut block = format!(
                "\n\n{START}\n{}\n",
                wiki(
                    &format!("{subject_name}/BigIdeas"),
                    "",
                    &format!("{subject_name} big ideas")
                )
            );
            for normalized in keys {
                if let Some(idea) = ideas.get(&normalized) {
                    block.push_str(&format!(
                        "- {}\n",
                        wiki(
                            &format!("{subject_name}/Big Ideas/{}", filename(&normalized)),
                            "",
                            &idea.name
                        )
                    ));
                }
            }
            block.push_str(END);
            block.push('\n');
            write_changed(&path, &format!("{}{block}", without_links(&raw)))?;
        }
        write_changed(&subject.join("BigIdeas.md"), &index)?;
        // Retain the historical filename as a working link, not a second registry.
        write_changed(
            &subject.join("Big Ideas.md"),
            &format!(
                "# {subject_name} big ideas\n\n{}\n",
                wiki(
                    &format!("{subject_name}/BigIdeas"),
                    "",
                    "Open the subject index"
                )
            ),
        )?;
        root.push_str(&format!(
            "- {}\n",
            wiki(&format!("{subject_name}/BigIdeas"), "", &subject_name)
        ));
    }
    write_changed(&vault.join("BigIdeas.md"), &root)?;
    Ok(())
}
#[derive(Deserialize)]
pub(super) struct DocumentQuery {
    path: String,
}
pub(super) async fn document(
    State(a): State<Arc<App>>,
    Query(q): Query<DocumentQuery>,
) -> R<Json<Value>> {
    let vault = std::fs::canonicalize(a.vault().await)?;
    let path = q.path.trim_end_matches(".md");
    let relative = Path::new(path);
    if relative.is_absolute()
        || relative
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        bail_document()?;
    }
    let mut candidate = vault.join(format!("{path}.md"));
    if !candidate.exists() && relative.components().count() == 1 {
        let mut matches = Vec::new();
        for subject in dirs(&vault) {
            let mut folders = vec![subject.join("Big Ideas")];
            folders.extend(
                dirs(&subject)
                    .into_iter()
                    .map(|course| course.join("Mind Map Notes")),
            );
            for folder in folders {
                if let Ok(entries) = std::fs::read_dir(folder) {
                    for entry in entries.filter_map(Result::ok) {
                        if entry
                            .path()
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .is_some_and(|s| s.eq_ignore_ascii_case(path))
                            && entry.path().extension().and_then(|s| s.to_str()) == Some("md")
                        {
                            matches.push(entry.path());
                        }
                    }
                }
            }
        }
        if matches.len() > 1 {
            return Err(anyhow!(
                "Several documents have this title; use a subject/course-qualified link"
            )
            .into());
        }
        if let Some(found) = matches.pop() {
            candidate = found;
        }
    }
    let target = std::fs::canonicalize(candidate)?;
    if !target.starts_with(&vault) || target.extension().and_then(|s| s.to_str()) != Some("md") {
        bail_document()?;
    }
    Ok(Json(
        json!({"path":target.strip_prefix(&vault)?.to_string_lossy(),"title":target.file_stem().unwrap().to_string_lossy(),"body":std::fs::read_to_string(target)?}),
    ))
}
fn bail_document() -> AResult<()> {
    bail!("Choose a Markdown document inside the configured vault")
}

pub(super) fn catalog(vault: &Path) -> AResult<Vec<Value>> {
    let mut rows = Vec::new();
    let source_pattern = regex::Regex::new(r"(?m)^Source: \[\[([^|\]]+)\|[^\]]*\]\]")?;
    for subject in dirs(vault) {
        let directory = subject.join("Big Ideas");
        if !directory.is_dir() {
            continue;
        }
        for file in std::fs::read_dir(directory)?.filter_map(Result::ok) {
            if file.path().is_symlink() {
                continue;
            }
            let raw = std::fs::read_to_string(file.path()).unwrap_or_default();
            if !raw.starts_with(&format!("---\n{GENERATED}\n")) {
                continue;
            }
            let title = raw.lines().find_map(|l| l.strip_prefix("# ")).unwrap_or("");
            let sources=source_pattern.captures_iter(&raw).map(|c|{
                let (path,heading)=c[1].split_once('#').unwrap_or((&c[1],""));
                json!({"path":path,"heading":heading,"title":Path::new(path).file_name().unwrap_or_default().to_string_lossy()})
            }).collect::<Vec<_>>();
            let relative = file
                .path()
                .strip_prefix(vault)?
                .to_string_lossy()
                .replace('\\', "/");
            rows.push(json!({"id":relative,"class_type":name(&subject),"name":title,"summary":format!("{} source sections across this subject",sources.len()),"concept_path":relative,"notes":sources.iter().filter_map(|s|s["title"].as_str()).collect::<Vec<_>>(),"sources":sources}));
        }
    }
    rows.sort_by_key(|v| {
        (
            v["class_type"].as_str().unwrap_or("").to_lowercase(),
            v["name"].as_str().unwrap_or("").to_lowercase(),
        )
    });
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "subject-ideas-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        root
    }
    fn note(root: &Path, course: &str, title: &str, body: &str) -> PathBuf {
        let path = root
            .join("Math")
            .join(course)
            .join("Mind Map Notes")
            .join(format!("{title}.md"));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, body).unwrap();
        path
    }
    #[test]
    fn subject_concepts_collect_cross_course_sections_and_keep_links_unambiguous() {
        let root = fixture();
        let first = note(
            &root,
            "Algebra",
            "Functions",
            "## Degree\nThe highest exponent.\n\n## Roots\nZeros of a function.\n",
        );
        note(
            &root,
            "Precalculus",
            "Functions",
            "## degree\nCompare the powers in standard form.\n\n```md\n## Not an idea\n```\n",
        );
        rebuild(&root).unwrap();
        let index = std::fs::read_to_string(root.join("Math/BigIdeas.md")).unwrap();
        assert_eq!(index.matches("Math/Big Ideas/degree|").count(), 1);
        assert!(index.contains("Math/Big Ideas/roots|"));
        let page = std::fs::read_to_string(root.join("Math/Big Ideas/degree.md")).unwrap();
        assert!(page.contains("The highest exponent."));
        assert!(page.contains("Compare the powers"));
        assert!(page.contains("[[Math/Algebra/Mind Map Notes/Functions#Degree|"));
        assert!(page.contains("[[Math/Precalculus/Mind Map Notes/Functions#degree|"));
        assert!(!root.join("Math/Big Ideas/not an idea.md").exists());
        let saved = std::fs::read_to_string(&first).unwrap();
        assert!(saved.contains("[[Math/Big Ideas/degree|Degree]]"));
        assert_eq!(saved.matches(START).count(), 1);
        rebuild(&root).unwrap();
        assert_eq!(std::fs::read_to_string(first).unwrap(), saved);
        let catalog = catalog(&root).unwrap();
        assert_eq!(catalog.len(), 2);
        assert_eq!(catalog[0]["sources"].as_array().unwrap().len(), 2);
        assert!(!integration::approved_body(&saved).contains(START));
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn regenerated_ideas_remove_old_contributions_and_preserve_handwritten_files() {
        let root = fixture();
        let source = note(&root, "Algebra", "Notes", "## Roots\nOld content.");
        rebuild(&root).unwrap();
        let manual = root.join("Math/Big Ideas/Personal.md");
        std::fs::write(&manual, "My own document").unwrap();
        std::fs::write(&source, "## Degree\nNew content.").unwrap();
        rebuild(&root).unwrap();
        assert!(!root.join("Math/Big Ideas/roots.md").exists());
        assert!(manual.exists());
        std::fs::remove_file(source).unwrap();
        rebuild(&root).unwrap();
        assert!(!root.join("Math/Big Ideas/degree.md").exists());
        assert!(catalog(&root).unwrap().is_empty());
        assert!(!std::fs::read_to_string(root.join("Math/BigIdeas.md"))
            .unwrap()
            .contains("degree|"));
        assert_ne!(filename("x^2"), filename("x2"));
        std::fs::remove_dir_all(root).unwrap();
    }
}
