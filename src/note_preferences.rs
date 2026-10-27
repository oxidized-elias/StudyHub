use super::*;

pub(super) fn schema(db: &Connection) -> AResult<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS note_preferences(session_id INTEGER PRIMARY KEY, note_style TEXT NOT NULL DEFAULT '');")?;
    Ok(())
}
pub(super) fn valid(style: &str) -> bool {
    ["", "bullet_points", "text_chunks", "hybrid"].contains(&style)
}
pub(super) fn get(a: &App, id: i64) -> String {
    a.db.lock()
        .unwrap()
        .query_row(
            "SELECT note_style FROM note_preferences WHERE session_id=?1",
            [id],
            |r| r.get(0),
        )
        .unwrap_or_default()
}
pub(super) fn save(a: &App, id: i64, style: &str) -> AResult<()> {
    if !valid(style) {
        bail!("Choose Settings Default, Bullet Points, Text Chunks, or Combined");
    }
    let db = a.db.lock().unwrap();
    let exists: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM sessions WHERE id=?1)",
        [id],
        |r| r.get(0),
    )?;
    if !exists {
        bail!("This note does not exist");
    }
    db.execute("INSERT INTO note_preferences(session_id,note_style) VALUES(?1,?2) ON CONFLICT(session_id) DO UPDATE SET note_style=excluded.note_style",params![id,style])?;
    Ok(())
}
pub(super) async fn read(State(a): State<Arc<App>>, AxPath(id): AxPath<i64>) -> R<Json<Value>> {
    load_row(&a, id)?;
    let style = get(&a, id);
    let default = a.settings.lock().await.note_style.clone();
    Ok(Json(
        json!({"note_style":style,"default_style":default,"effective_style":if style.is_empty(){&default}else{&style}}),
    ))
}
pub(super) async fn write(
    State(a): State<Arc<App>>,
    AxPath(id): AxPath<i64>,
    Json(v): Json<Value>,
) -> R<Json<Value>> {
    let style = v["note_style"]
        .as_str()
        .ok_or_else(|| anyhow!("Note Style is required"))?;
    save(&a, id, style)?;
    read(State(a), AxPath(id)).await
}
