use crate::*;
#[derive(Clone, Copy, Default)]
pub struct Scope {
    pub job: Option<i64>,
    pub thread: Option<i64>,
}
pub fn schema(db: &Connection) -> AResult<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS runtime_logs(id INTEGER PRIMARY KEY AUTOINCREMENT,job_id INTEGER,thread_id INTEGER,created INTEGER NOT NULL,stage TEXT NOT NULL,message TEXT NOT NULL,detail TEXT NOT NULL);CREATE INDEX IF NOT EXISTS runtime_job ON runtime_logs(job_id,id);CREATE INDEX IF NOT EXISTS runtime_thread ON runtime_logs(thread_id,id);")?;
    Ok(())
}
pub fn log(a: &App, scope: Scope, stage: &str, message: &str, detail: Value) {
    if scope.job.is_none() && scope.thread.is_none() {
        return;
    }
    let _=a.db.lock().unwrap().execute("INSERT INTO runtime_logs(job_id,thread_id,created,stage,message,detail)VALUES(?1,?2,?3,?4,?5,?6)",params![scope.job,scope.thread,now(),stage,message,detail.to_string()]);
}
#[derive(Deserialize)]
pub struct Filter {
    pub job_id: Option<i64>,
    pub thread_id: Option<i64>,
    pub after: Option<i64>,
}
pub async fn get(State(a): State<Arc<App>>, Query(q): Query<Filter>) -> R<Json<Value>> {
    if q.job_id.is_none() && q.thread_id.is_none() {
        return Err(anyhow!("Choose a job or conversation to view its logs").into());
    }
    let db = a.db.lock().unwrap();
    let mut st=db.prepare("SELECT id,created,stage,message,detail FROM runtime_logs WHERE id>?1 AND ((?2 IS NOT NULL AND job_id=?2) OR (?3 IS NOT NULL AND thread_id=?3)) ORDER BY id LIMIT 250")?;
    let rows=st.query_map(params![q.after.unwrap_or(0),q.job_id,q.thread_id],|r|{let raw:String=r.get(4)?;Ok(json!({"id":r.get::<_,i64>(0)?,"created":r.get::<_,i64>(1)?,"stage":r.get::<_,String>(2)?,"message":r.get::<_,String>(3)?,"detail":serde_json::from_str::<Value>(&raw).unwrap_or(json!({}))}))})?.collect::<Result<Vec<_>,_>>()?;
    Ok(Json(json!({"entries":rows})))
}
pub fn safe_error(text: &str, key: &str) -> String {
    let mut text = text.to_string();
    if !key.is_empty() {
        text = text.replace(key, "[redacted]");
    }
    let tokens =
        regex::Regex::new(r"(?i)(bearer\s+)[A-Za-z0-9._~+/=-]+|\bsk-[A-Za-z0-9_-]+").unwrap();
    tokens
        .replace_all(&text, "[redacted]")
        .chars()
        .filter(|c| !c.is_control() || *c == '\n')
        .take(1000)
        .collect()
}
pub fn provider_error(body: &str, key: &str) -> Option<String> {
    let v: Value = serde_json::from_str(body).ok()?;
    v["error"]
        .as_str()
        .or(v["error"]["message"].as_str())
        .map(|s| safe_error(s, key))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn provider_errors_are_bounded_and_redacted() {
        assert_eq!(
            provider_error(r#"{"error":"model not found"}"#, ""),
            Some("model not found".into())
        );
        let error = provider_error(
            r#"{"error":{"message":"failed secret-key Bearer another-token sk-sensitive"}}"#,
            "secret-key",
        )
        .unwrap();
        assert!(!error.contains("secret-key"));
        assert!(!error.contains("another-token"));
        assert!(!error.contains("sk-sensitive"));
        assert!(provider_error("<html>gateway error</html>", "").is_none());
        assert_eq!(safe_error(&"x".repeat(4000), "").len(), 1000);
    }
    #[tokio::test]
    async fn logs_are_incremental_and_scoped() {
        let db = Connection::open_in_memory().unwrap();
        schema(&db).unwrap();
        let a = Arc::new(App {
            db: Mutex::new(db),
            http: reqwest::Client::new(),
            dir: std::env::temp_dir(),
            locks: Mutex::new(HashSet::new()),
        });
        log(
            &a,
            Scope {
                job: Some(1),
                thread: Some(2),
            },
            "start",
            "request",
            json!({"model":"tutor"}),
        );
        log(
            &a,
            Scope {
                job: Some(9),
                thread: Some(9),
            },
            "start",
            "private",
            json!({}),
        );
        let rows = get(
            State(a.clone()),
            Query(Filter {
                job_id: Some(1),
                thread_id: None,
                after: None,
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(rows["entries"].as_array().unwrap().len(), 1);
        let after = rows["entries"][0]["id"].as_i64();
        assert!(get(
            State(a.clone()),
            Query(Filter {
                job_id: Some(1),
                thread_id: None,
                after
            })
        )
        .await
        .unwrap()
        .0["entries"]
            .as_array()
            .unwrap()
            .is_empty());
        assert!(get(
            State(a),
            Query(Filter {
                job_id: None,
                thread_id: None,
                after: None
            })
        )
        .await
        .is_err());
    }
}
