//! Integration tests for the library server API.

use bytes::Bytes;
use ebook_converter_library_server::{api_routes, config::ServerConfig, AppState};
use http_body_util::{BodyExt, Full};
use http::Request;
use tower::ServiceExt;

async fn body_to_bytes<B>(body: B) -> Bytes
where
    B: http_body::Body<Data = Bytes> + Unpin,
    B::Error: std::fmt::Debug + std::fmt::Display,
{
    body.collect().await.unwrap().to_bytes()
}

#[tokio::test]
async fn get_capabilities_returns_200_and_json() {
    let dir = tempfile::tempdir().unwrap();
    let config = ServerConfig {
        library_path: dir.path().to_path_buf(),
        bind: "127.0.0.1:0".to_string(),
    };
    let state = AppState::new(config).await;
    let app = api_routes(state);

    let req = Request::builder()
        .uri("/api/capabilities")
        .body(Full::<Bytes>::new(Bytes::new()))
        .unwrap();
    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), 200);
    let (_, body) = response.into_parts();
    let body = body_to_bytes(body).await;
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["list"], true);
    assert_eq!(json["get"], true);
    assert_eq!(json["put"], true);
}

#[tokio::test]
async fn get_entries_empty_returns_200_and_empty_list() {
    let dir = tempfile::tempdir().unwrap();
    let config = ServerConfig {
        library_path: dir.path().to_path_buf(),
        bind: "127.0.0.1:0".to_string(),
    };
    let state = AppState::new(config).await;
    let app = api_routes(state);

    let req = Request::builder()
        .uri("/api/entries")
        .body(Full::<Bytes>::new(Bytes::new()))
        .unwrap();
    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), 200);
    let (_, body) = response.into_parts();
    let body = body_to_bytes(body).await;
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["entries"].is_array());
    assert_eq!(json["entries"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn put_entries_returns_200_and_id() {
    let dir = tempfile::tempdir().unwrap();
    let config = ServerConfig {
        library_path: dir.path().to_path_buf(),
        bind: "127.0.0.1:0".to_string(),
    };
    let state = AppState::new(config).await;
    let app = api_routes(state);

    let req = Request::builder()
        .uri("/api/entries")
        .method("PUT")
        .body(Full::new(Bytes::from_static(b"fake epub content")))
        .unwrap();
    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), 200);
    let (_, body) = response.into_parts();
    let body = body_to_bytes(body).await;
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["id"].as_str().unwrap().ends_with(".epub"));
}
