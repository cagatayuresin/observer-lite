use std::convert::Infallible;

use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::{header, StatusCode};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use bytes::Bytes;
use serde::Deserialize;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;

use crate::auth::decode_token;
use crate::error::AppError;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/api/sse/dashboard", get(dashboard))
}

#[derive(Deserialize)]
struct TokenQuery {
    token: String,
}

async fn dashboard(
    State(state): State<AppState>,
    Query(query): Query<TokenQuery>,
) -> Result<Response, AppError> {
    let claims = decode_token(&state.settings, &query.token)
        .map_err(|_| AppError::new(StatusCode::UNAUTHORIZED, "Invalid token"))?;
    if claims.typ != "access" {
        return Err(AppError::new(StatusCode::UNAUTHORIZED, "Invalid token"));
    }
    let receiver = state.sse.subscribe();
    let stream = async_stream(receiver);
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/event-stream")
        .header(header::CACHE_CONTROL, "no-cache")
        .body(Body::from_stream(stream))
        .map_err(|_| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, "SSE failed"))
}

fn async_stream(
    receiver: Option<tokio::sync::mpsc::Receiver<String>>,
) -> impl tokio_stream::Stream<Item = Result<Bytes, Infallible>> + Send {
    let (_fallback_tx, fallback_rx) = tokio::sync::mpsc::channel(1);
    let receiver = receiver.unwrap_or(fallback_rx);
    ReceiverStream::new(receiver).map(|message| Ok(Bytes::from(message)))
}
