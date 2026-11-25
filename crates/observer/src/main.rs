use std::net::SocketAddr;

use observer::{bootstrap, router, Settings};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let settings = Settings::from_env();
    let port = settings.port;
    let state = match bootstrap(settings).await {
        Ok(state) => state,
        Err(err) => {
            tracing::error!(error = %err.detail, "startup failed");
            std::process::exit(1);
        }
    };
    let app = router(state.clone());
    let address = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = match tokio::net::TcpListener::bind(address).await {
        Ok(listener) => listener,
        Err(err) => {
            tracing::error!(error = %err, %address, "bind failed");
            std::process::exit(1);
        }
    };
    tracing::info!(%address, version = %state.settings.version, "Observer Lite listening");
    let scheduler = state.scheduler.clone();
    let shutdown = async move {
        let _ = tokio::signal::ctrl_c().await;
        scheduler.shutdown();
    };
    if let Err(err) = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown)
    .await
    {
        tracing::error!(error = %err, "server stopped");
        std::process::exit(1);
    }
}
