//! HTTP API routes matching the library standard (docs/library-standard.md).

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use axum::extract::Request;
use bytes::Bytes;

use ebook_converter_core::library::{LibraryCapabilities, LibraryEntry, ListOptions, ListResult};

use crate::AppState;

/// Query params for GET /api/entries
#[derive(Debug, serde::Deserialize)]
pub struct ListQuery {
    pub page: Option<u64>,
    pub limit: Option<u64>,
    pub q: Option<String>,
    pub format: Option<String>,
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/api/capabilities", get(capabilities))
        .route("/api/entries", get(list_entries).put(put_entry))
        .route("/api/entries/{id}", get(get_entry_meta).delete(delete_entry))
        .route("/api/entries/{id}/file", get(get_entry_file))
        .with_state(state)
}

async fn capabilities(State(state): State<AppState>, _req: Request) -> Json<LibraryCapabilities> {
    Json(state.store.capabilities())
}

async fn list_entries(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
    _req: Request,
) -> Result<Json<ListResult>, ApiError> {
    let page = q.page.unwrap_or(1).saturating_sub(1);
    let limit = q.limit.unwrap_or(50).min(500);
    let offset = page.saturating_mul(limit);
    let opts = ListOptions {
        offset: Some(offset),
        limit: Some(limit),
        query: q.q,
        format: q.format,
    };
    let result = state.store.list(&opts).map_err(ApiError::from)?;
    Ok(Json(result))
}

async fn get_entry_meta(
    State(state): State<AppState>,
    Path(id): Path<String>,
    _req: Request,
) -> Result<Json<LibraryEntry>, ApiError> {
    let decoded = url_decode(&id)?;
    let list = state.store.list(&ListOptions::default()).map_err(ApiError::from)?;
    let entry = list
        .entries
        .into_iter()
        .find(|e| e.id == decoded)
        .ok_or_else(|| ApiError::NotFound(decoded))?;
    Ok(Json(entry))
}

async fn get_entry_file(
    State(state): State<AppState>,
    Path(id): Path<String>,
    _req: Request,
) -> Result<impl IntoResponse, ApiError> {
    let decoded = url_decode(&id)?;
    let (data, format) = state.store.get(&decoded).map_err(ApiError::from)?;
    let content_type = mime_for_format(&format);
    Ok((
        [(axum::http::header::CONTENT_TYPE, content_type)],
        data,
    ))
}

async fn put_entry(
    State(state): State<AppState>,
    body: Bytes,
) -> Result<Json<PutResponse>, ApiError> {
    let id = state
        .store
        .put(&body, None, Some("epub"))
        .map_err(ApiError::from)?;
    Ok(Json(PutResponse { id }))
}

async fn delete_entry(
    State(state): State<AppState>,
    Path(id): Path<String>,
    _req: Request,
) -> Result<StatusCode, ApiError> {
    let decoded = url_decode(&id)?;
    state.store.delete(&decoded).map_err(ApiError::from)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(serde::Serialize)]
struct PutResponse {
    id: String,
}

fn mime_for_format(format: &str) -> &'static str {
    match format.to_lowercase().as_str() {
        "epub" => "application/epub+zip",
        "pdf" => "application/pdf",
        "txt" | "text" => "text/plain",
        "html" => "text/html",
        "md" => "text/markdown",
        _ => "application/octet-stream",
    }
}

fn url_decode(id: &str) -> Result<String, ApiError> {
    percent_encoding::percent_decode_str(id)
        .decode_utf8()
        .map(|s| s.to_string())
        .map_err(|_| ApiError::BadRequest("invalid id encoding".to_string()))
}

#[derive(Debug)]
enum ApiError {
    Io(std::io::Error),
    NotFound(String),
    BadRequest(String),
}

impl From<std::io::Error> for ApiError {
    fn from(e: std::io::Error) -> Self {
        if e.kind() == std::io::ErrorKind::NotFound {
            ApiError::NotFound("entry not found".to_string())
        } else {
            ApiError::Io(e)
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, body) = match &self {
            ApiError::NotFound(s) => (StatusCode::NOT_FOUND, s.clone()),
            ApiError::BadRequest(s) => (StatusCode::BAD_REQUEST, s.clone()),
            ApiError::Io(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        };
        (status, body).into_response()
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::Io(e) => write!(f, "{}", e),
            ApiError::NotFound(s) => write!(f, "not found: {}", s),
            ApiError::BadRequest(s) => write!(f, "bad request: {}", s),
        }
    }
}
