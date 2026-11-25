mod admin;
mod auth;
mod channels;
mod groups;
mod incidents;
mod keys;
mod maintenance;
mod monitors;
mod read;
mod sse;
mod staticfiles;
mod users;

use axum::extract::Request;
use axum::http::{HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use axum::Router;

use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(auth::router())
        .merge(admin::router())
        .merge(channels::router())
        .merge(groups::router())
        .merge(incidents::router())
        .merge(keys::router())
        .merge(maintenance::router())
        .merge(monitors::router())
        .merge(read::router())
        .merge(sse::router())
        .merge(users::router())
        .fallback(staticfiles::spa)
        .layer(axum::middleware::from_fn(security_headers))
        .with_state(state)
}

async fn security_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let unauthorized = response.status() == StatusCode::UNAUTHORIZED;
    let headers = response.headers_mut();
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
    if unauthorized {
        headers.insert("www-authenticate", HeaderValue::from_static("Bearer"));
    }
    response
}
