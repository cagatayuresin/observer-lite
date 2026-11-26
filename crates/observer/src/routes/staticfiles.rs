use axum::body::Body;
use axum::extract::Request;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../../frontend/dist/"]
struct Assets;

pub async fn spa(request: Request) -> Response {
    let path = request.uri().path().trim_start_matches('/');
    if path.is_empty() {
        return file("index.html");
    }
    if Assets::get(path).is_some() {
        return file(path);
    }
    file("index.html")
}

fn file(path: &str) -> Response {
    let Some(asset) = Assets::get(path) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    Response::builder()
        .header(header::CONTENT_TYPE, mime.as_ref())
        .body(Body::from(asset.data.into_owned()))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}
