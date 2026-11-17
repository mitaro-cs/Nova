//! Ошибки интерфейса и плеера у зрителей: пишем в журнал приложения (без адресов и секретов).

use crate::App;
use crate::auth::Viewer;
use crate::error::{ApiError, ApiResult};
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct ClientLog {
    kind: String,
    message: String,
    #[serde(default, rename = "where")]
    place: String,
}

pub async fn client(State(app): State<Arc<App>>, v: Viewer, Json(b): Json<ClientLog>) -> ApiResult<StatusCode> {
    if !matches!(b.kind.as_str(), "error" | "rejection" | "player") {
        return Err(ApiError::bad("bad_kind", "Неизвестный вид сообщения"));
    }
    let key = format!("diag:{}", v.profile.id);
    if app.diag_limit.blocked(&key) {
        return Err(ApiError::new(StatusCode::TOO_MANY_REQUESTS, "rate_limited", "Слишком часто - подождите минуту"));
    }
    app.diag_limit.fail(&key);
    let place: String = b.place.chars().take(120).collect();
    tracing::warn!(target: "client", "{}: {} ({})", b.kind, crate::logs::sanitize(&b.message), crate::logs::sanitize(&place));
    Ok(StatusCode::NO_CONTENT)
}
