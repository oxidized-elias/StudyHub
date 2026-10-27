use anyhow::Result;
use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
    middleware::{from_fn, from_fn_with_state, Next},
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde_json::json;

fn inject(html: &str, prefix: &str) -> String {
    html.replacen(
        "<head>",
        &format!(
            "<head><script>window.__APP_BASE__={};window.__EMBEDDED__=true;</script>",
            json!(prefix)
        ),
        1,
    )
}
async fn embedded(req: Request<Body>, next: Next, prefix: &'static str) -> Response {
    let mut response = next.run(req).await;
    let html = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("text/html"));
    if html {
        let (mut parts, body) = response.into_parts();
        match to_bytes(body, 8 * 1024 * 1024).await {
            Ok(bytes) => {
                parts.headers.remove(header::CONTENT_LENGTH);
                response = Response::from_parts(
                    parts,
                    Body::from(inject(&String::from_utf8_lossy(&bytes), prefix)),
                );
            }
            Err(_) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Could not render application",
                )
                    .into_response()
            }
        }
    }
    response
}
async fn authenticate(
    axum::extract::State(expected): axum::extract::State<Option<String>>,
    req: Request<Body>,
    next: Next,
) -> Response {
    if let Some(expected) = expected {
        if req
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            != Some(expected.as_str())
        {
            return (
                StatusCode::UNAUTHORIZED,
                [(
                    header::WWW_AUTHENTICATE,
                    "Basic realm=\"StudyHub\", charset=\"UTF-8\"",
                )],
                "Sign in to your study workspace",
            )
                .into_response();
        }
    }
    next.run(req).await
}
fn browser_reset(html: &str, epoch: &str) -> String {
    // Only app-owned preferences/notifications are reset after a server data reset.
    let script = format!(
        r#"<head><script>try{{const epoch={};if(localStorage.getItem('studyhub.data-epoch')!==epoch){{for(const storage of [localStorage,sessionStorage]){{for(const key of Object.keys(storage)){{if(key.startsWith('hub.')||key.startsWith('study.')||key.startsWith('dismissed-warning.'))storage.removeItem(key)}}}}localStorage.setItem('studyhub.data-epoch',epoch)}}}}catch{{}}</script>"#,
        json!(epoch)
    );
    html.replacen("<head>", &script, 1)
}
fn data_epoch() -> Result<String> {
    let dir = std::path::PathBuf::from(std::env::var("DATA_DIR").unwrap_or_else(|_| "data".into()));
    let path = dir.join(".browser-epoch");
    if path.exists() {
        return Ok(std::fs::read_to_string(path)?);
    }
    let epoch = format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    );
    std::fs::write(path, &epoch)?;
    Ok(epoch)
}
fn router(notes: Router, learning: Router, authorization: Option<String>, epoch: &str) -> Router {
    let html = browser_reset(include_str!("../web/index.html"), epoch);
    let notes = notes.layer(from_fn(|req, next| embedded(req, next, "/notes")));
    let learning = learning.layer(from_fn(|req, next| embedded(req, next, "/learn")));
    Router::new()
        .route("/", get(move || { let html = html.clone(); async move { Html(html) } }))
        .route("/api/health", get(|| async { Json(json!({"service":"studyhub","notes":true,"learning":true,"ok":true,"unified":true})) }))
        .route("/notes/", get(|| async { Redirect::temporary("/notes") }))
        .route("/learn/", get(|| async { Redirect::temporary("/learn") }))
        .nest("/notes", notes)
        .nest("/learn", learning)
        .layer(from_fn_with_state(authorization, authenticate))
}
async fn shutdown() {
    #[cfg(unix)]
    {
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("install SIGTERM handler");
        tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = term.recv() => {} }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}
async fn run() -> Result<()> {
    let bind = std::env::var("STUDYHUB_BIND")
        .or_else(|_| std::env::var("HUB_BIND"))
        .unwrap_or_else(|_| "127.0.0.1:8082".into());
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    let notes = notes_web::create_router().await?;
    let learning = selfhosted_ai_tests::create_router(notes.clone()).await?;
    let authorization = std::env::var("STUDYHUB_PASSWORD")
        .ok()
        .filter(|s| !s.is_empty())
        .map(|password| {
            format!(
                "Basic {}",
                BASE64.encode(format!(
                    "{}:{password}",
                    std::env::var("STUDYHUB_USER").unwrap_or_else(|_| "admin".into())
                ))
            )
        });
    println!("StudyHub: http://{bind} (one server; notes and learning integrated)");
    axum::serve(
        listener,
        router(notes, learning, authorization, &data_epoch()?),
    )
    .with_graceful_shutdown(shutdown())
    .await?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use tower::ServiceExt;
    async fn request(app: Router, path: &str, auth: bool) -> Response {
        let mut req = Request::builder().uri(path);
        if auth {
            req = req.header(header::AUTHORIZATION, "Basic test");
        }
        app.oneshot(req.body(Body::empty()).unwrap()).await.unwrap()
    }
    #[tokio::test]
    async fn unified_routes_render_with_prefixes_and_authenticate_every_module() {
        let module = || {
            Router::new()
                .route(
                    "/",
                    get(|| async { Html("<html><head></head><body>module</body></html>") }),
                )
                .route("/api/health", get(|| async { Json(json!({"ok":true})) }))
        };
        let app = router(module(), module(), Some("Basic test".into()), "test");
        for path in [
            "/",
            "/api/health",
            "/notes",
            "/notes/api/health",
            "/learn",
            "/learn/api/health",
        ] {
            assert_eq!(
                request(app.clone(), path, false).await.status(),
                StatusCode::UNAUTHORIZED
            );
            assert_eq!(
                request(app.clone(), path, true).await.status(),
                StatusCode::OK,
                "{path}"
            );
        }
        let shell = request(app.clone(), "/", true).await;
        let shell_body = to_bytes(shell.into_body(), 1_000_000).await.unwrap();
        assert!(String::from_utf8_lossy(&shell_body).contains("studyhub.data-epoch"));
        for prefix in ["/notes", "/learn"] {
            let response = request(app.clone(), prefix, true).await;
            let body = to_bytes(response.into_body(), 10000).await.unwrap();
            assert!(String::from_utf8_lossy(&body)
                .contains(&format!("window.__APP_BASE__=\"{prefix}\"")));
        }
    }
    #[tokio::test]
    async fn integrated_notes_configuration_and_import_work_without_http_servers() {
        let temp = std::env::temp_dir().join(format!(
            "studyhub-unified-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::env::set_var("DATA_DIR", &temp);
        std::env::set_var("VAULT_DIR", temp.join("vault"));
        std::env::set_var("LEARNING_DATA_DIR", temp.join("learning"));
        std::env::set_var("NOTES_INTEGRATION_TOKEN", "test-internal-token");
        let notes = notes_web::create_router().await.unwrap();
        let learn = selfhosted_ai_tests::create_router(notes.clone())
            .await
            .unwrap();
        let app = router(notes, learn, Some("Basic test".into()), "test");
        async fn json_request(
            app: Router,
            method: &str,
            path: &str,
            value: serde_json::Value,
        ) -> serde_json::Value {
            let req = Request::builder()
                .method(method)
                .uri(path)
                .header(header::AUTHORIZATION, "Basic test")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(value.to_string()))
                .unwrap();
            let res = app.oneshot(req).await.unwrap();
            let status = res.status();
            let bytes = to_bytes(res.into_body(), 1_000_000).await.unwrap();
            assert_eq!(
                status,
                StatusCode::OK,
                "{}",
                String::from_utf8_lossy(&bytes)
            );
            serde_json::from_slice(&bytes).unwrap()
        }
        let config = json_request(app.clone(), "GET", "/learn/api/notes/config", json!({})).await;
        assert_eq!(
            config["vault_path"],
            temp.join("vault").to_string_lossy().as_ref()
        );
        let saved = json_request(
            app.clone(),
            "PUT",
            "/learn/api/notes/config",
            json!({"vault_path":temp.join("vault"),"note_style":"hybrid"}),
        )
        .await;
        assert!(saved.is_object());
        let config = json_request(app.clone(), "GET", "/learn/api/notes/config", json!({})).await;
        assert_eq!(config["note_style"], "hybrid");
        assert_eq!(
            json_request(app.clone(), "GET", "/learn/api/notes", json!({})).await,
            json!([])
        );
        let relative = "Topic/Course/Mind Map Notes/Example.md";
        let note_path = temp.join("vault").join(relative);
        std::fs::create_dir_all(note_path.parent().unwrap()).unwrap();
        std::fs::write(&note_path, "## Method\nHuman-approved source text.\n").unwrap();
        let original = temp.join("vault/Topic/Course/Uploaded Notes/source.md");
        std::fs::create_dir_all(original.parent().unwrap()).unwrap();
        std::fs::write(original, "Original upload").unwrap();
        {
            let db = rusqlite::Connection::open(temp.join("review/sessions.db")).unwrap();
            db.execute("INSERT INTO sessions(id,created,class_type,course,unit,lesson,title,note_path,pdf,status)VALUES(1,1,'Topic','Course','','Method','Example',?1,'source.md','done')",[relative]).unwrap();
        }
        let notebook = json_request(
            app.clone(),
            "POST",
            "/learn/api/notebooks",
            json!({"title":"Test notebook","goal":"Test","instructions":""}),
        )
        .await;
        let id = notebook["id"].as_i64().unwrap();
        let imported = json_request(
            app.clone(),
            "POST",
            &format!("/learn/api/notebooks/{id}/import"),
            json!({"note_ids":[1]}),
        )
        .await;
        assert_eq!(imported["imported"].as_array().unwrap().len(), 1);
        let notebook = json_request(
            app.clone(),
            "GET",
            &format!("/learn/api/notebooks/{id}"),
            json!({}),
        )
        .await;
        assert!(notebook["sources"][0]["body"]
            .as_str()
            .unwrap()
            .contains("Human-approved source text"));
        let original = request(app.clone(), "/learn/api/notes/1/pdf/source.md", true).await;
        assert_eq!(original.status(), StatusCode::OK);
        assert_eq!(
            to_bytes(original.into_body(), 1000).await.unwrap().as_ref(),
            b"Original upload"
        );
        let document = json_request(
            app.clone(),
            "GET",
            "/notes/api/vault/document?path=Topic%2FCourse%2FMind%20Map%20Notes%2FExample",
            json!({}),
        )
        .await;
        assert!(document["body"]
            .as_str()
            .unwrap()
            .contains("Human-approved source text"));
        let denied = request(
            app.clone(),
            "/notes/api/vault/document?path=..%2Fconfig",
            true,
        )
        .await;
        assert_eq!(denied.status(), StatusCode::BAD_REQUEST);
        let settings = json_request(app, "GET", "/learn/api/settings", json!({})).await;
        assert_eq!(settings["unified"], true);
        std::fs::remove_dir_all(temp).unwrap();
    }
}
