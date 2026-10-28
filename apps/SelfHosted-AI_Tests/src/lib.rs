use anyhow::{anyhow, bail, ensure, Context, Result as AResult};
use axum::{
    extract::{DefaultBodyLimit, Multipart, Path as AxPath, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
mod history;
mod learning;
mod providers;
mod runtime;
mod study;
mod web_source;
use providers::Provider;
struct App {
    db: Mutex<Connection>,
    http: reqwest::Client,
    dir: PathBuf,
    locks: Mutex<HashSet<i64>>,
}
#[derive(Debug)]
struct AppErr(anyhow::Error);
impl<E: Into<anyhow::Error>> From<E> for AppErr {
    fn from(e: E) -> Self {
        Self(e.into())
    }
}
impl IntoResponse for AppErr {
    fn into_response(self) -> Response {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":self.0.to_string()})),
        )
            .into_response()
    }
}
type R<T> = Result<T, AppErr>;
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
fn parse(s: &str) -> AResult<Value> {
    let s = s.trim();
    let s = s
        .strip_prefix("```json")
        .or_else(|| s.strip_prefix("```"))
        .unwrap_or(s)
        .trim();
    Ok(serde_json::from_str(
        s.strip_suffix("```").unwrap_or(s).trim(),
    )?)
}
fn schema(db: &Connection) -> AResult<()> {
    db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;
CREATE TABLE IF NOT EXISTS settings(key TEXT PRIMARY KEY,value TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS providers(id INTEGER PRIMARY KEY AUTOINCREMENT,data TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS notebooks(id INTEGER PRIMARY KEY AUTOINCREMENT,title TEXT NOT NULL,goal TEXT NOT NULL,instructions TEXT NOT NULL,created INTEGER NOT NULL,archived INTEGER NOT NULL DEFAULT 0);
CREATE TABLE IF NOT EXISTS sources(id INTEGER PRIMARY KEY AUTOINCREMENT,notebook_id INTEGER NOT NULL REFERENCES notebooks(id),title TEXT NOT NULL,body TEXT NOT NULL,note_id INTEGER,version TEXT NOT NULL DEFAULT '',teach INTEGER NOT NULL DEFAULT 1,quiz INTEGER NOT NULL DEFAULT 1,created INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS uploads(source_id INTEGER PRIMARY KEY REFERENCES sources(id) ON DELETE CASCADE,path TEXT NOT NULL,mime TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS threads(id INTEGER PRIMARY KEY AUTOINCREMENT,notebook_id INTEGER REFERENCES notebooks(id),mode TEXT NOT NULL,title TEXT NOT NULL,context TEXT NOT NULL,provider_id INTEGER NOT NULL,created INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS messages(id INTEGER PRIMARY KEY AUTOINCREMENT,thread_id INTEGER NOT NULL REFERENCES threads(id),role TEXT NOT NULL,body TEXT NOT NULL,created INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS artifacts(id INTEGER PRIMARY KEY AUTOINCREMENT,thread_id INTEGER NOT NULL REFERENCES threads(id),notebook_id INTEGER REFERENCES notebooks(id),kind TEXT NOT NULL,title TEXT NOT NULL,content TEXT NOT NULL,prompt TEXT NOT NULL,provider_id INTEGER NOT NULL,created INTEGER NOT NULL,share_token TEXT UNIQUE);
CREATE TABLE IF NOT EXISTS attempts(id INTEGER PRIMARY KEY AUTOINCREMENT,artifact_id INTEGER NOT NULL REFERENCES artifacts(id),state TEXT NOT NULL,answers TEXT NOT NULL,deadline INTEGER,finished INTEGER,result TEXT NOT NULL DEFAULT '{}',created INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS jobs(id INTEGER PRIMARY KEY AUTOINCREMENT,status TEXT NOT NULL,request TEXT NOT NULL,artifact_id INTEGER,error TEXT NOT NULL DEFAULT '',progress INTEGER NOT NULL DEFAULT 0,created INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS events(id INTEGER PRIMARY KEY AUTOINCREMENT,kind TEXT NOT NULL,entity INTEGER,detail TEXT NOT NULL,created INTEGER NOT NULL);
CREATE INDEX IF NOT EXISTS message_thread ON messages(thread_id,id);
CREATE INDEX IF NOT EXISTS source_notebook ON sources(notebook_id);")?;
    learning::schema(db)?;
    runtime::schema(db)?;
    let has_archive = {
        let mut st = db.prepare("PRAGMA table_info(notebooks)")?;
        let names = st
            .query_map([], |r| r.get::<_, String>(1))?
            .collect::<Result<Vec<_>, _>>()?;
        names.iter().any(|n| n == "archived")
    };
    if !has_archive {
        db.execute(
            "ALTER TABLE notebooks ADD COLUMN archived INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    db.execute("UPDATE jobs SET status='failed',error='Generation was interrupted by a server restart. Retry this job; earlier history is preserved.' WHERE status='running'",[])?;
    Ok(())
}
fn setting(a: &App, key: &str, default: &str) -> String {
    a.db.lock()
        .unwrap()
        .query_row("SELECT value FROM settings WHERE key=?1", [key], |r| {
            r.get(0)
        })
        .optional()
        .ok()
        .flatten()
        .unwrap_or_else(|| default.into())
}
fn event(a: &App, kind: &str, id: i64, detail: &Value) {
    let _ = a.db.lock().unwrap().execute(
        "INSERT INTO events(kind,entity,detail,created)VALUES(?1,?2,?3,?4)",
        params![kind, id, detail.to_string(), now()],
    );
}
fn provider(a: &App, id: i64) -> AResult<Provider> {
    let raw: String =
        a.db.lock()
            .unwrap()
            .query_row("SELECT data FROM providers WHERE id=?1", [id], |r| r.get(0))
            .context("Select an AI provider in Settings")?;
    let mut p: Provider = serde_json::from_str(&raw)?;
    p.id = id;
    Ok(p)
}
static NOTES_ROUTER: std::sync::OnceLock<Router> = std::sync::OnceLock::new();
pub async fn create_router(notes: Router) -> AResult<Router> {
    NOTES_ROUTER
        .set(notes)
        .map_err(|_| anyhow!("Learning module already initialized"))?;
    let dir = PathBuf::from(
        std::env::var("LEARNING_DATA_DIR").unwrap_or_else(|_| "data/learning".into()),
    );
    std::fs::create_dir_all(&dir)?;
    let db = Connection::open(dir.join("learning.db"))?;
    schema(&db)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))?;
        std::fs::set_permissions(
            dir.join("learning.db"),
            std::fs::Permissions::from_mode(0o600),
        )?;
    }
    let a = Arc::new(App {
        db: Mutex::new(db),
        http: reqwest::Client::builder()
            .timeout(Duration::from_secs(900))
            .redirect(reqwest::redirect::Policy::none())
            .build()?,
        dir,
        locks: Mutex::new(HashSet::new()),
    });
    Ok(router(a))
}

fn router(a: Arc<App>) -> Router {
    Router::new()
        .route(
            "/",
            get(|| async { Html(include_str!("../web/index.html")) }),
        )
        .route(
            "/api/health",
            get(|| async { Json(json!({"service":"learning","ok":true})) }),
        )
        .route("/api/settings", get(settings_get).put(settings_put))
        .route("/api/providers", get(providers_get).post(providers_put))
        .route("/api/providers/:id/test", post(provider_test))
        .route("/api/providers/:id/diagnostics", get(provider_diagnostics))
        .route("/api/providers/models", post(provider_models))
        .route("/api/notes", get(notes_list))
        .route("/api/notes/:id", get(notes_detail))
        .route("/api/notes/:id/source/:source", get(notes_source))
        .route("/api/notes/:id/pdf/:name", get(notes_pdf))
        .route("/api/notes/folders", get(notes_folders))
        .route(
            "/api/notes/config",
            get(notes_config_get).put(notes_config_put),
        )
        .route("/api/notebooks", get(notebooks_get).post(notebook_create))
        .route(
            "/api/notebooks/:id",
            get(notebook_get)
                .put(notebook_update)
                .delete(notebook_archive),
        )
        .route("/api/notebooks/:id/import", post(notes_import))
        .route("/api/notebooks/:id/sources", post(source_add))
        .route("/api/notebooks/:id/upload", post(source_upload))
        .route("/api/sources/:id/file", get(source_file))
        .route(
            "/api/sources/:id",
            axum::routing::put(source_update).delete(source_delete),
        )
        .route("/api/notebooks/:id/sync", post(notes_sync))
        .route(
            "/api/threads",
            get(study::threads_get).post(study::thread_create),
        )
        .route(
            "/api/threads/:id",
            get(study::thread_get).put(study::thread_move),
        )
        .route("/api/threads/:id/chat", post(study::chat))
        .route("/api/generate", post(study::generate))
        .route("/api/jobs", get(study::jobs_get))
        .route("/api/runtime/logs", get(runtime::get))
        .route("/api/jobs/:id/retry", post(study::job_retry))
        .route("/api/jobs/:id/cancel", post(study::job_cancel))
        .route(
            "/api/artifacts/:id",
            get(study::artifact_get).put(study::artifact_edit),
        )
        .route("/api/artifacts/:id/export", get(study::artifact_export))
        .route(
            "/api/artifacts/:id/share",
            post(study::share_create).delete(study::share_delete),
        )
        .route("/api/shared/:token", get(study::shared_get))
        .route("/api/artifacts/:id/attempts", post(study::attempt_create))
        .route(
            "/api/attempts/:id",
            get(study::attempt_get).put(study::attempt_state),
        )
        .route("/api/attempts/:id/answer", post(study::answer))
        .route(
            "/api/attempts/:id/questions/:question/journal",
            get(learning::journal_read).post(learning::journal_save),
        )
        .route(
            "/api/attempts/:id/questions/:question/section",
            post(learning::reveal),
        )
        .route(
            "/api/attempts/:id/questions/:question/hint",
            post(learning::hint),
        )
        .route(
            "/api/attempts/:id/questions/:question/step",
            post(learning::step),
        )
        .route("/api/journal", get(learning::journal_get))
        .route("/api/notebooks/:id/mastery", get(learning::mastery_get))
        .route(
            "/api/threads/:id/corrections",
            get(learning::corrections_get).post(learning::correction_create),
        )
        .route(
            "/api/corrections/:id/apply",
            post(learning::correction_apply),
        )
        .route("/api/attempts/:id/finish", post(study::finish))
        .route("/api/attempts/:id/analyze", post(study::analyze))
        .route("/api/history", get(study::history))
        .route(
            "/api/history/:kind/:id",
            axum::routing::delete(history::delete),
        )
        .route("/api/progress", get(study::progress))
        .layer(DefaultBodyLimit::max(64 * 1024 * 1024))
        .with_state(a)
}
async fn settings_get(State(a): State<Arc<App>>) -> Json<Value> {
    Json(
        json!({"unified":true,"notes_url":"/notes/","notes_token_set":false,"active_provider":setting(&a,"active_provider","0")}),
    )
}
async fn settings_put(State(a): State<Arc<App>>, Json(v): Json<Value>) -> R<Json<Value>> {
    if let Some(value) = v.get("active_provider") {
        let id = value
            .as_i64()
            .or_else(|| value.as_str().and_then(|s| s.parse().ok()))
            .ok_or_else(|| anyhow!("Choose a saved model profile"))?;
        if id <= 0 {
            return Err(anyhow!("Choose a saved model profile").into());
        }
        provider(&a, id)?;
        a.db.lock().unwrap().execute("INSERT INTO settings(key,value)VALUES('active_provider',?1)ON CONFLICT(key)DO UPDATE SET value=excluded.value",[id.to_string()])?;
    }
    Ok(Json(json!({"ok":true})))
}

async fn providers_get(State(a): State<Arc<App>>) -> R<Json<Value>> {
    let db = a.db.lock().unwrap();
    let mut st = db.prepare("SELECT id,data FROM providers ORDER BY id")?;
    let rows = st
        .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    let rows = rows
        .into_iter()
        .map(|(id, s)| {
            let mut p: Provider = serde_json::from_str(&s)?;
            p.id = id;
            Ok(p.public())
        })
        .collect::<AResult<Vec<_>>>()?;
    Ok(Json(json!(rows)))
}
async fn providers_put(State(a): State<Arc<App>>, Json(mut p): Json<Provider>) -> R<Json<Value>> {
    p.label = p.label.trim().to_string();
    p.model = p.model.trim().to_string();
    p.base_url = p.base_url.trim().to_string();
    if p.id > 0 && p.api_key.is_empty() {
        let old = provider(&a, p.id)?;
        if old.kind == p.kind && old.base_url == p.base_url {
            p.api_key = old.api_key;
        }
    }
    p.validate()?;
    let mut db = a.db.lock().unwrap();
    let transaction = db.transaction()?;
    if p.id > 0 {
        if transaction.execute(
            "UPDATE providers SET data=?1 WHERE id=?2",
            params![serde_json::to_string(&p)?, p.id],
        )? == 0
        {
            return Err(anyhow!("Provider profile not found").into());
        }
    } else {
        transaction.execute(
            "INSERT INTO providers(data)VALUES(?1)",
            [serde_json::to_string(&p)?],
        )?;
        p.id = transaction.last_insert_rowid();
    }
    transaction.execute("INSERT INTO settings(key,value)VALUES('active_provider',?1)ON CONFLICT(key)DO UPDATE SET value=excluded.value",[p.id.to_string()])?;
    transaction.commit()?;
    Ok(Json(p.public()))
}
async fn provider_models(State(a): State<Arc<App>>, Json(mut p): Json<Provider>) -> R<Json<Value>> {
    if !["ollama", "compatible"].contains(&p.kind.as_str()) {
        return Err(anyhow!(
            "Automatic model discovery is available for Ollama and compatible servers"
        )
        .into());
    }
    if p.id > 0 && p.api_key.is_empty() {
        let old = provider(&a, p.id)?;
        if old.kind == p.kind && old.base_url == p.base_url {
            p.api_key = old.api_key;
        }
    }
    // Model discovery precedes model/name selection. Validate the connection independently.
    p.model = "discovery".into();
    p.label = "discovery".into();
    p.validate()?;
    let suffix = if p.kind == "ollama" {
        "api/tags"
    } else {
        "models"
    };
    let mut req = a
        .http
        .get(format!("{}/{suffix}", p.base_url.trim_end_matches('/')))
        .timeout(Duration::from_secs(10));
    if !p.api_key.is_empty() {
        req = req.bearer_auth(&p.api_key);
    }
    let response = req
        .send()
        .await
        .context("Cannot load models from this server")?;
    if !response.status().is_success() {
        return Err(anyhow!("Model discovery failed with HTTP {}", response.status()).into());
    }
    let value: Value = response.json().await?;
    let models = providers::model_names(&p.kind, &value)?;
    Ok(Json(json!({"models":models})))
}
async fn provider_diagnostics(
    State(a): State<Arc<App>>,
    AxPath(id): AxPath<i64>,
) -> R<Json<Value>> {
    let p = provider(&a, id)?;
    p.validate()?;
    if p.kind != "ollama" {
        return Ok(Json(
            json!({"model":p.model,"provider":p.kind,"message":"Detailed load status is available for Ollama. Other endpoints report request progress and HTTP errors in Live Session Logs."}),
        ));
    }
    async fn read(a: &App, p: &Provider, path: &str) -> AResult<Value> {
        let mut req = a
            .http
            .get(format!("{}/api/{path}", p.base_url.trim_end_matches('/')))
            .timeout(Duration::from_secs(5));
        if !p.api_key.is_empty() {
            req = req.bearer_auth(&p.api_key);
        }
        let r = req
            .send()
            .await
            .map_err(|e| anyhow!("{}", e.without_url()))?;
        ensure!(
            r.status().is_success(),
            "Ollama {path} returned HTTP {}",
            r.status()
        );
        Ok(r.json().await?)
    }
    let (version, tags, ps) = tokio::join!(
        read(&a, &p, "version"),
        read(&a, &p, "tags"),
        read(&a, &p, "ps")
    );
    let mut errors = vec![];
    for (name, r) in [
        ("version", &version),
        ("installed models", &tags),
        ("loaded models", &ps),
    ] {
        if let Err(e) = r {
            errors.push(format!(
                "{name}: {}",
                runtime::safe_error(&e.to_string(), &p.api_key)
            ));
        }
    }
    let matches = |v: &Value| v["name"] == p.model || v["model"] == p.model;
    let installed = tags
        .as_ref()
        .ok()
        .and_then(|v| v["models"].as_array())
        .map(|rows| rows.iter().any(matches));
    let loaded = ps
        .as_ref()
        .ok()
        .and_then(|v| v["models"].as_array())
        .map(|rows| rows.iter().any(matches));
    let running=ps.as_ref().ok().and_then(|v|v["models"].as_array()).map(|rows|rows.iter().map(|m|json!({"name":m["name"],"size":m["size"],"size_vram":m["size_vram"],"context_length":m["context_length"],"expires_at":m["expires_at"]})).collect::<Vec<_>>()).unwrap_or_default();
    let message = if installed == Some(false) {
        "The configured model is not listed as installed on this Ollama server. Install the exact model there or select an installed model in Settings."
    } else if loaded == Some(false) {
        "The model is not currently listed as loaded. It may be loading or unloaded; inspect the live request log and the Ollama server log for the reason."
    } else if loaded == Some(true) {
        "Ollama reports this model loaded. A pending request may still be processing the prompt or generating a response."
    } else {
        "Could not determine model state. Check the connection errors below."
    };
    Ok(Json(
        json!({"provider":p.kind,"model":p.model,"context_configured":p.context,"server_version":version.ok().map(|v|v["version"].clone()),"installed":installed,"loaded":loaded,"running_models":running,"errors":errors,"message":message}),
    ))
}
async fn provider_test(State(a): State<Arc<App>>, AxPath(id): AxPath<i64>) -> R<Json<Value>> {
    let p = provider(&a, id)?;
    let response = providers::call(&a, &p, "Reply briefly.", "Reply with OK.", false).await?;
    Ok(Json(json!({"ok":true,"response":response})))
}
async fn notes_exchange(
    method: axum::http::Method,
    path: &str,
    body: Option<Value>,
) -> AResult<Response> {
    use tower::ServiceExt;
    let router = NOTES_ROUTER
        .get()
        .ok_or_else(|| anyhow!("Notes module is unavailable"))?;
    let mut request = axum::http::Request::builder().method(method).uri(path);
    let token = std::env::var("NOTES_INTEGRATION_TOKEN").unwrap_or_default();
    if !token.is_empty() {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let payload = if let Some(body) = body {
        request = request.header("content-type", "application/json");
        axum::body::Body::from(serde_json::to_vec(&body)?)
    } else {
        axum::body::Body::empty()
    };
    Ok(router
        .clone()
        .oneshot(request.body(payload)?)
        .await
        .unwrap())
}
async fn notes_request(_a: &App, path: &str) -> AResult<Value> {
    let response = notes_exchange(axum::http::Method::GET, path, None).await?;
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), 64 * 1024 * 1024).await?;
    decode_notes_response(status, &body)
}

fn decode_notes_response(status: reqwest::StatusCode, body: &[u8]) -> AResult<Value> {
    if !status.is_success() {
        let detail = serde_json::from_slice::<Value>(body)
            .ok()
            .and_then(|v| v["error"].as_str().map(str::to_owned))
            .unwrap_or_else(|| "Check the NoteSorter server and integration settings.".into());
        bail!("NoteSorter returned HTTP {status}: {detail}");
    }
    Ok(serde_json::from_slice(body)?)
}
async fn notes_list(State(a): State<Arc<App>>) -> R<Json<Value>> {
    Ok(Json(notes_request(&a, "/api/integration/notes").await?))
}
async fn notebooks_get(State(a): State<Arc<App>>) -> R<Json<Value>> {
    let db = a.db.lock().unwrap();
    let mut st=db.prepare("SELECT id,title,goal,instructions,created,(SELECT COUNT(*) FROM sources WHERE notebook_id=n.id) FROM notebooks n WHERE archived=0 ORDER BY id DESC")?;
    let rows=st.query_map([],|r|Ok(json!({"id":r.get::<_,i64>(0)?,"title":r.get::<_,String>(1)?,"goal":r.get::<_,String>(2)?,"instructions":r.get::<_,String>(3)?,"created":r.get::<_,i64>(4)?,"source_count":r.get::<_,i64>(5)?})))?.collect::<Result<Vec<_>,_>>()?;
    Ok(Json(json!(rows)))
}
async fn notebook_create(State(a): State<Arc<App>>, Json(v): Json<Value>) -> R<Json<Value>> {
    let title = v["title"].as_str().unwrap_or("").trim();
    if title.is_empty() {
        return Err(anyhow!("Name your notebook").into());
    }
    let db = a.db.lock().unwrap();
    db.execute(
        "INSERT INTO notebooks(title,goal,instructions,created)VALUES(?1,?2,?3,?4)",
        params![
            title,
            v["goal"].as_str().unwrap_or(""),
            v["instructions"].as_str().unwrap_or(""),
            now()
        ],
    )?;
    Ok(Json(json!({"id":db.last_insert_rowid()})))
}
async fn notebook_get(State(a): State<Arc<App>>, AxPath(id): AxPath<i64>) -> R<Json<Value>> {
    let db = a.db.lock().unwrap();
    let mut v:Value=db.query_row("SELECT id,title,goal,instructions,created FROM notebooks WHERE id=?1",[id],|r|Ok(json!({"id":r.get::<_,i64>(0)?,"title":r.get::<_,String>(1)?,"goal":r.get::<_,String>(2)?,"instructions":r.get::<_,String>(3)?,"created":r.get::<_,i64>(4)?})))?;
    let mut st=db.prepare("SELECT id,title,body,note_id,version,teach,quiz FROM sources WHERE notebook_id=?1 ORDER BY id")?;
    v["sources"]=json!(st.query_map([id],|r|Ok(json!({"id":r.get::<_,i64>(0)?,"title":r.get::<_,String>(1)?,"body":r.get::<_,String>(2)?,"note_id":r.get::<_,Option<i64>>(3)?,"version":r.get::<_,String>(4)?,"teach":r.get::<_,i64>(5)?==1,"quiz":r.get::<_,i64>(6)?==1})))?.collect::<Result<Vec<_>,_>>()?);
    Ok(Json(v))
}
async fn notebook_update(
    State(a): State<Arc<App>>,
    AxPath(id): AxPath<i64>,
    Json(v): Json<Value>,
) -> R<Json<Value>> {
    let title = v["title"].as_str().unwrap_or("").trim();
    if title.is_empty() {
        return Err(anyhow!("Notebook title is required").into());
    }
    a.db.lock().unwrap().execute(
        "UPDATE notebooks SET title=?1,goal=?2,instructions=?3 WHERE id=?4",
        params![
            title,
            v["goal"].as_str().unwrap_or(""),
            v["instructions"].as_str().unwrap_or(""),
            id
        ],
    )?;
    Ok(Json(json!({"ok":true})))
}
async fn notebook_archive(State(a): State<Arc<App>>, AxPath(id): AxPath<i64>) -> R<Json<Value>> {
    a.db.lock()
        .unwrap()
        .execute("UPDATE notebooks SET archived=1 WHERE id=?1", [id])?;
    event(
        &a,
        "notebook_archived",
        id,
        &json!({"history_preserved":true}),
    );
    Ok(Json(json!({"archived":true})))
}
fn insert_source(
    a: &App,
    notebook: i64,
    title: &str,
    body: &str,
    note_id: Option<i64>,
    version: &str,
) -> AResult<i64> {
    ensure!(
        !title.trim().is_empty() && !body.trim().is_empty(),
        "Source needs a title and readable content"
    );
    ensure!(
        body.len() <= 2_000_000,
        "Source text is too large; split it into smaller documents"
    );
    let db = a.db.lock().unwrap();
    if let Some(nid) = note_id {
        if let Some(id) = db
            .query_row(
                "SELECT id FROM sources WHERE notebook_id=?1 AND note_id=?2",
                params![notebook, nid],
                |r| r.get::<_, i64>(0),
            )
            .optional()?
        {
            db.execute(
                "UPDATE sources SET title=?1,body=?2,version=?3 WHERE id=?4",
                params![title, body, version, id],
            )?;
            return Ok(id);
        }
    }
    db.execute("INSERT INTO sources(notebook_id,title,body,note_id,version,created)VALUES(?1,?2,?3,?4,?5,?6)",params![notebook,title,body,note_id,version,now()])?;
    Ok(db.last_insert_rowid())
}
async fn notes_import(
    State(a): State<Arc<App>>,
    AxPath(id): AxPath<i64>,
    Json(v): Json<Value>,
) -> R<Json<Value>> {
    let ids = v["note_ids"]
        .as_array()
        .ok_or_else(|| anyhow!("Select notes to import"))?;
    let mut imported = vec![];
    for nid in ids {
        let nid = nid.as_i64().ok_or_else(|| anyhow!("Invalid note ID"))?;
        let note = notes_request(&a, &format!("/api/integration/notes/{nid}")).await?;
        imported.push(insert_source(
            &a,
            id,
            note["title"].as_str().unwrap_or("Note"),
            note["body"].as_str().unwrap_or(""),
            Some(nid),
            note["version"].as_str().unwrap_or(""),
        )?);
    }
    event(&a, "notes_imported", id, &json!({"source_ids":imported}));
    Ok(Json(json!({"imported":imported})))
}
async fn refresh_imported(a: &App, notebook: Option<i64>) -> AResult<()> {
    let Some(id) = notebook else { return Ok(()) };
    let ids = {
        let db = a.db.lock().unwrap();
        let mut st =
            db.prepare("SELECT note_id FROM sources WHERE notebook_id=?1 AND note_id IS NOT NULL")?;
        let rows = st
            .query_map([id], |r| r.get::<_, i64>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        rows
    };
    for nid in ids {
        let note=notes_request(a,&format!("/api/integration/notes/{nid}")).await.context("Could not refresh imported notes. Start NoteSorter or disable Refresh imported notes to use the saved notebook sources")?;
        insert_source(
            a,
            id,
            note["title"].as_str().unwrap_or("Note"),
            note["body"].as_str().unwrap_or(""),
            Some(nid),
            note["version"].as_str().unwrap_or(""),
        )?;
    }
    Ok(())
}
async fn notes_sync(State(a): State<Arc<App>>, AxPath(id): AxPath<i64>) -> R<Json<Value>> {
    let ids = {
        let db = a.db.lock().unwrap();
        let mut st =
            db.prepare("SELECT note_id FROM sources WHERE notebook_id=?1 AND note_id IS NOT NULL")?;
        let ids = st
            .query_map([id], |r| r.get::<_, i64>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        ids
    };
    notes_import(State(a), AxPath(id), Json(json!({"note_ids":ids}))).await
}
async fn source_add(
    State(a): State<Arc<App>>,
    AxPath(id): AxPath<i64>,
    Json(v): Json<Value>,
) -> R<Json<Value>> {
    let body = if let Some(url) = v["url"].as_str().filter(|s| !s.trim().is_empty()) {
        web_source::fetch(url).await?
    } else {
        v["body"].as_str().unwrap_or("").to_string()
    };
    let title = v["title"]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| v["url"].as_str())
        .unwrap_or("");
    let sid = insert_source(&a, id, title, &body, None, "")?;
    Ok(Json(json!({"id":sid})))
}
async fn source_update(
    State(a): State<Arc<App>>,
    AxPath(id): AxPath<i64>,
    Json(v): Json<Value>,
) -> R<Json<Value>> {
    let db = a.db.lock().unwrap();
    for key in ["teach", "quiz"] {
        if let Some(b) = v[key].as_bool() {
            db.execute(
                &format!("UPDATE sources SET {key}=?1 WHERE id=?2"),
                params![b as i64, id],
            )?;
        }
    }
    for key in ["title", "body"] {
        if let Some(s) = v[key].as_str() {
            if s.trim().is_empty() || s.len() > 2_000_000 {
                return Err(anyhow!("Source content must be nonempty and at most 2 MB").into());
            }
            db.execute(
                &format!("UPDATE sources SET {key}=?1 WHERE id=?2"),
                params![s, id],
            )?;
        }
    }
    Ok(Json(json!({"ok":true})))
}
async fn source_delete(State(a): State<Arc<App>>, AxPath(id): AxPath<i64>) -> R<Json<Value>> {
    a.db.lock()
        .unwrap()
        .execute("DELETE FROM sources WHERE id=?1", [id])?;
    let _ = std::fs::remove_dir_all(a.dir.join("sources").join(id.to_string()));
    Ok(Json(json!({"ok":true})))
}
async fn source_upload(
    State(a): State<Arc<App>>,
    AxPath(id): AxPath<i64>,
    mut mp: Multipart,
) -> R<Json<Value>> {
    let mut ids = vec![];
    while let Some(f) = mp.next_field().await? {
        let name = f.file_name().unwrap_or("source.txt").to_string();
        let bytes = f.bytes().await?;
        let ext = Path::new(&name)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let body = if ext == "pdf" {
            let unique: String =
                a.db.lock()
                    .unwrap()
                    .query_row("SELECT lower(hex(randomblob(12)))", [], |r| r.get(0))?;
            let temp = a.dir.join(format!("upload-{unique}.pdf"));
            std::fs::write(&temp, &bytes)?;
            let out = tokio::process::Command::new("pdftotext")
                .arg("-layout")
                .arg(&temp)
                .arg("-")
                .output()
                .await;
            let _ = std::fs::remove_file(temp);
            let out=out.context("PDF sources require Poppler pdftotext; scanned images should first be processed in NoteSorter")?;
            if !out.status.success() {
                return Err(anyhow!(
                    "Could not extract this PDF; use NoteSorter for scanned pages"
                )
                .into());
            }
            String::from_utf8_lossy(&out.stdout).to_string()
        } else {
            if !["md", "markdown", "txt", "csv", "json"].contains(&ext.as_str()) {
                return Err(anyhow!(
                    "Upload PDF, Markdown, text, CSV or JSON. Use NoteSorter to transcribe images."
                )
                .into());
            }
            String::from_utf8(bytes.to_vec()).context("Text sources must use UTF-8")?
        };
        let source = insert_source(&a, id, &name, &body, None, "")?;
        let dir = a.dir.join("sources").join(source.to_string());
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join("original"), &bytes)?;
        a.db.lock().unwrap().execute(
            "INSERT INTO uploads(source_id,path,mime)VALUES(?1,?2,?3)",
            params![
                source,
                format!("sources/{source}/original"),
                if ext == "pdf" {
                    "application/pdf"
                } else {
                    "text/plain; charset=utf-8"
                }
            ],
        )?;
        ids.push(source);
    }
    Ok(Json(json!({"source_ids":ids})))
}

async fn source_file(State(a): State<Arc<App>>, AxPath(id): AxPath<i64>) -> R<Response> {
    let raw: Option<(String, String)> =
        a.db.lock()
            .unwrap()
            .query_row(
                "SELECT path,mime FROM uploads WHERE source_id=?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
    let (bytes, mime) = if let Some((path, mime)) = raw {
        (std::fs::read(a.dir.join(path))?, mime)
    } else {
        let body: String =
            a.db.lock()
                .unwrap()
                .query_row("SELECT body FROM sources WHERE id=?1", [id], |r| r.get(0))?;
        (body.into_bytes(), "text/plain; charset=utf-8".into())
    };
    Ok((
        [
            (axum::http::header::CONTENT_TYPE, mime),
            (
                axum::http::header::X_CONTENT_TYPE_OPTIONS,
                "nosniff".to_string(),
            ),
        ],
        bytes,
    )
        .into_response())
}

async fn notes_config_get(State(a): State<Arc<App>>) -> R<Json<Value>> {
    Ok(Json(notes_request(&a, "/api/integration/config").await?))
}
async fn notes_config_put(State(_a): State<Arc<App>>, Json(v): Json<Value>) -> R<Json<Value>> {
    let response =
        notes_exchange(axum::http::Method::PUT, "/api/integration/config", Some(v)).await?;
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), 64 * 1024 * 1024).await?;
    Ok(Json(decode_notes_response(status, &body)?))
}

async fn notes_detail(State(a): State<Arc<App>>, AxPath(id): AxPath<i64>) -> R<Json<Value>> {
    let mut v = notes_request(&a, &format!("/api/integration/notes/{id}")).await?;
    if let Some(sources) = v["sources"].as_array_mut() {
        for source in sources {
            if let Some(url) = source["url"].as_str() {
                source["url"] = json!(if let Some(name) = url.strip_prefix("/pdf/") {
                    format!("/api/notes/{id}/pdf/{name}")
                } else {
                    url.replace("/api/review/session/", "/api/notes/")
                });
            }
        }
    }
    Ok(Json(v))
}
#[derive(Deserialize)]
struct FolderQuery {
    path: Option<String>,
}
async fn notes_folders(State(a): State<Arc<App>>, Query(q): Query<FolderQuery>) -> R<Json<Value>> {
    let mut path = String::from("/api/integration/folders");
    if let Some(p) = q.path {
        path.push_str("?path=");
        path.push_str(&url_encode(&p));
    }
    Ok(Json(notes_request(&a, &path).await?))
}
async fn notes_source(
    State(a): State<Arc<App>>,
    AxPath((id, source)): AxPath<(i64, i64)>,
) -> R<Response> {
    notes_binary(&a, &format!("/api/review/session/{id}/source/{source}")).await
}
async fn notes_pdf(
    State(a): State<Arc<App>>,
    AxPath((id, name)): AxPath<(i64, String)>,
) -> R<Response> {
    notes_binary(
        &a,
        &format!("/api/review/session/{id}/pdf/{}", url_encode(&name)),
    )
    .await
}
async fn notes_binary(_a: &App, path: &str) -> R<Response> {
    let response = notes_exchange(axum::http::Method::GET, path, None).await?;
    if !response.status().is_success() {
        return Err(anyhow!(
            "Could not open the uploaded source: HTTP {}",
            response.status()
        )
        .into());
    }
    Ok(response)
}

fn url_encode(s: &str) -> String {
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

#[cfg(test)]
mod notes_response_tests {
    use super::*;
    #[test]
    fn integration_errors_keep_the_upstream_reason() {
        let error = decode_notes_response(
            reqwest::StatusCode::BAD_REQUEST,
            br#"{"error":"Approved note file is missing: example.md"}"#,
        )
        .unwrap_err();
        assert!(error
            .to_string()
            .contains("Approved note file is missing: example.md"));
        assert!(
            decode_notes_response(reqwest::StatusCode::BAD_GATEWAY, b"not json")
                .unwrap_err()
                .to_string()
                .contains("502")
        );
        assert_eq!(
            decode_notes_response(reqwest::StatusCode::OK, br#"{"id":3}"#).unwrap()["id"],
            3
        );
    }
}

#[cfg(test)]
mod profile_persistence_tests {
    use super::*;
    fn open(dir: &Path) -> Arc<App> {
        std::fs::create_dir_all(dir).unwrap();
        let db = Connection::open(dir.join("learning.db")).unwrap();
        schema(&db).unwrap();
        Arc::new(App {
            db: Mutex::new(db),
            http: reqwest::Client::new(),
            dir: dir.into(),
            locks: Mutex::new(HashSet::new()),
        })
    }
    #[tokio::test]
    async fn profile_edit_stays_saved_and_active_after_reopening_database() {
        let dir = std::env::temp_dir().join(format!(
            "studyhub-profile-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let app = open(&dir);
        let p = Provider {
            id: 0,
            label: " My local model ".into(),
            kind: "ollama".into(),
            base_url: "http://127.0.0.1:11434".into(),
            model: " first-model ".into(),
            api_key: "test-stored-token".into(),
            context: 8192,
        };
        let saved = providers_put(State(app.clone()), Json(p)).await.unwrap().0;
        let id = saved["id"].as_i64().unwrap();
        assert_eq!(saved["model"], "first-model");
        assert!(saved.get("api_key").is_none());
        let edited = Provider {
            id,
            label: "Local model".into(),
            kind: "ollama".into(),
            base_url: "http://127.0.0.1:11434".into(),
            model: "gemma4:e4b".into(),
            api_key: String::new(),
            context: 16384,
        };
        let _ = providers_put(State(app.clone()), Json(edited))
            .await
            .unwrap();
        assert_eq!(provider(&app, id).unwrap().api_key, "test-stored-token");
        let second = Provider {
            id: 0,
            label: "Other".into(),
            kind: "ollama".into(),
            base_url: "http://127.0.0.1:11434".into(),
            model: "other-model".into(),
            context: 8192,
            ..Provider::default()
        };
        let second = providers_put(State(app.clone()), Json(second))
            .await
            .unwrap()
            .0["id"]
            .as_i64()
            .unwrap();
        assert_eq!(setting(&app, "active_provider", "0"), second.to_string());
        let _ = settings_put(State(app.clone()), Json(json!({"active_provider":id})))
            .await
            .unwrap();
        assert!(
            settings_put(State(app.clone()), Json(json!({"active_provider":99999})))
                .await
                .is_err()
        );
        drop(app);
        let reopened = open(&dir);
        assert_eq!(setting(&reopened, "active_provider", "0"), id.to_string());
        assert_eq!(provider(&reopened, id).unwrap().model, "gemma4:e4b");
        let profiles = providers_get(State(reopened.clone())).await.unwrap().0;
        assert_eq!(profiles.as_array().unwrap().len(), 2);
        assert_eq!(profiles[0]["api_key_set"], true);
        assert!(!profiles.to_string().contains("test-stored-token"));
        drop(reopened);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
