use crate::*;

// Explicit transactions keep deletions consistent with the existing non-cascading schema.
fn remove_attempt(db: &Connection, id: i64) -> AResult<()> {
    db.execute("DELETE FROM problem_journal WHERE attempt_id=?1", [id])?;
    db.execute("DELETE FROM attempts WHERE id=?1", [id])?;
    Ok(())
}
fn remove_artifact(db: &Connection, id: i64) -> AResult<()> {
    let ids = {
        let mut st = db.prepare("SELECT id FROM attempts WHERE artifact_id=?1")?;
        let rows = st
            .query_map([id], |r| r.get::<_, i64>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        rows
    };
    for id in ids {
        remove_attempt(db, id)?;
    }
    db.execute(
        "DELETE FROM corrections WHERE artifact_id=?1 OR applied_artifact_id=?1",
        [id],
    )?;
    db.execute(
        "UPDATE jobs SET artifact_id=NULL WHERE artifact_id=?1",
        [id],
    )?;
    db.execute("DELETE FROM artifacts WHERE id=?1", [id])?;
    Ok(())
}
fn erase(a: &App, kind: &str, id: i64, data: &Value) -> AResult<()> {
    // A provider response must not save new children after their parent is removed.
    let active = a.locks.lock().unwrap();
    ensure!(
        active.is_empty(),
        "Wait for the current study request to finish before deleting history"
    );
    let mut db = a.db.lock().unwrap();
    let tx = db.transaction()?;
    match kind {
        "material" => {
            ensure!(
                tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM artifacts WHERE id=?1)",
                    [id],
                    |r| r.get::<_, bool>(0)
                )?,
                "Material no longer exists"
            );
            ensure!(
                !tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM jobs WHERE status='running')",
                    [],
                    |r| r.get::<_, bool>(0)
                )?,
                "Wait for generation to finish before deleting materials or conversations"
            );
            remove_artifact(&tx, id)?;
        }
        "conversation" => {
            ensure!(
                tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM threads WHERE id=?1)",
                    [id],
                    |r| r.get::<_, bool>(0)
                )?,
                "Conversation no longer exists"
            );
            ensure!(
                !tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM jobs WHERE status='running')",
                    [],
                    |r| r.get::<_, bool>(0)
                )?,
                "Wait for generation to finish before deleting materials or conversations"
            );
            let ids = {
                let mut st = tx.prepare("SELECT id FROM artifacts WHERE thread_id=?1")?;
                let rows = st
                    .query_map([id], |r| r.get::<_, i64>(0))?
                    .collect::<Result<Vec<_>, _>>()?;
                rows
            };
            for id in ids {
                remove_artifact(&tx, id)?;
            }
            tx.execute("DELETE FROM corrections WHERE thread_id=?1", [id])?;
            tx.execute("DELETE FROM messages WHERE thread_id=?1", [id])?;
            tx.execute("DELETE FROM runtime_logs WHERE thread_id=?1", [id])?;
            tx.execute("DELETE FROM threads WHERE id=?1", [id])?;
        }
        "session" => {
            remove_attempt(&tx, id)?;
        }
        "job" => {
            ensure!(
                !tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM jobs WHERE id=?1 AND status='running')",
                    [id],
                    |r| r.get::<_, bool>(0)
                )?,
                "Wait for generation to finish before deleting this job"
            );
            tx.execute("DELETE FROM runtime_logs WHERE job_id=?1", [id])?;
            tx.execute("DELETE FROM jobs WHERE id=?1", [id])?;
        }
        "journal" => {
            let question = data["question_id"]
                .as_i64()
                .ok_or_else(|| anyhow!("Question ID is required"))?;
            tx.execute(
                "DELETE FROM problem_journal WHERE attempt_id=?1 AND question_id=?2",
                params![id, question],
            )?;
        }
        "progress" => {
            // id=0 resets all progress; otherwise reset just the selected notebook.
            ensure!(id >= 0, "Invalid notebook ID");
            let ids = {
                let mut st=tx.prepare("SELECT t.id FROM attempts t JOIN artifacts a ON a.id=t.artifact_id WHERE ?1=0 OR a.notebook_id=?1")?;
                let rows = st
                    .query_map([id], |r| r.get::<_, i64>(0))?
                    .collect::<Result<Vec<_>, _>>()?;
                rows
            };
            for id in ids {
                remove_attempt(&tx, id)?;
            }
        }
        _ => bail!("Unknown history item type"),
    }
    tx.commit()?;
    Ok(())
}
pub async fn delete(
    State(a): State<Arc<App>>,
    AxPath((kind, id)): AxPath<(String, i64)>,
    Json(data): Json<Value>,
) -> R<Json<Value>> {
    erase(&a, &kind, id, &data)?;
    Ok(Json(json!({"deleted":true})))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn app() -> App {
        let db = Connection::open_in_memory().unwrap();
        schema(&db).unwrap();
        db.execute_batch("INSERT INTO notebooks VALUES(1,'Math','','',0,0);INSERT INTO notebooks VALUES(2,'Science','','',0,0);
        INSERT INTO threads VALUES(1,1,'quiz','Math','{}',1,0);INSERT INTO threads VALUES(2,2,'quiz','Science','{}',1,0);
        INSERT INTO artifacts VALUES(1,1,1,'quiz','A','{}','',1,0,'shared');INSERT INTO artifacts VALUES(2,2,2,'quiz','B','{}','',1,0,NULL);
        INSERT INTO attempts VALUES(1,1,'{}','{}',NULL,NULL,'{}',0);INSERT INTO attempts VALUES(2,2,'{}','{}',NULL,NULL,'{}',0);
        INSERT INTO problem_journal VALUES(1,0,'Keep',0);INSERT INTO problem_journal VALUES(2,0,'Other',0);
        INSERT INTO messages VALUES(1,1,'user','Hello',0);INSERT INTO messages VALUES(2,2,'user','Other',0);
        INSERT INTO jobs VALUES(1,'done','{}',1,'',100,0);").unwrap();
        App {
            db: Mutex::new(db),
            http: reqwest::Client::new(),
            dir: std::env::temp_dir(),
            locks: Mutex::new(HashSet::new()),
        }
    }
    fn count(a: &App, table: &str) -> i64 {
        a.db.lock()
            .unwrap()
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .unwrap()
    }
    #[test]
    fn deleting_parents_removes_dependencies_and_keeps_other_history() {
        let a = app();
        a.db.lock()
            .unwrap()
            .execute(
                "INSERT INTO corrections VALUES(1,1,1,0,1,'Wrong','proposed','{}','{}',NULL,0)",
                [],
            )
            .unwrap();
        erase(&a, "conversation", 1, &json!({})).unwrap();
        for table in [
            "threads",
            "artifacts",
            "attempts",
            "problem_journal",
            "messages",
        ] {
            assert_eq!(count(&a, table), 1);
        }
        assert_eq!(count(&a, "corrections"), 0);
        assert_eq!(count(&a, "notebooks"), 2);
        assert!(a
            .db
            .lock()
            .unwrap()
            .query_row("SELECT artifact_id FROM jobs WHERE id=1", [], |r| r
                .get::<_, Option<i64>>(0))
            .unwrap()
            .is_none());
        assert_eq!(
            a.db.lock()
                .unwrap()
                .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| r
                    .get::<_, i64>(
                    0
                ))
                .unwrap(),
            0
        );
    }
    #[test]
    fn leaf_deletions_and_notebook_progress_reset_are_scoped() {
        let a = app();
        erase(&a, "journal", 1, &json!({"question_id":0})).unwrap();
        assert_eq!(count(&a, "attempts"), 2);
        erase(&a, "job", 1, &json!({})).unwrap();
        assert_eq!(count(&a, "artifacts"), 2);
        erase(&a, "progress", 1, &json!({})).unwrap();
        assert_eq!(count(&a, "attempts"), 1);
        assert_eq!(count(&a, "problem_journal"), 1);
        erase(&a, "session", 2, &json!({})).unwrap();
        assert_eq!(count(&a, "attempts"), 0);
        assert_eq!(count(&a, "artifacts"), 2);
        erase(&a, "material", 1, &json!({})).unwrap();
        assert_eq!(count(&a, "threads"), 2);
        assert_eq!(count(&a, "artifacts"), 1);
    }
    #[test]
    fn active_work_and_invalid_requests_leave_history_untouched() {
        let a = app();
        a.db.lock()
            .unwrap()
            .execute("UPDATE jobs SET status='running'", [])
            .unwrap();
        assert!(erase(&a, "job", 1, &json!({})).is_err());
        assert!(erase(&a, "conversation", 1, &json!({})).is_err());
        assert!(erase(&a, "unknown", 1, &json!({})).is_err());
        assert_eq!(count(&a, "threads"), 2);
        a.locks.lock().unwrap().insert(5);
        assert!(erase(&a, "session", 1, &json!({})).is_err());
        assert_eq!(count(&a, "attempts"), 2);
    }
}
