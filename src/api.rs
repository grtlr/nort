use std::net::SocketAddr;

use axum::{routing::get, Router};

use crate::shutdown::ShutdownSignal;

async fn root() -> &'static str {
    "Hello, World!"
}

pub async fn start(shutdown: ShutdownSignal) {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3001));
        let routes = Router::new().route("/", get(root));

        tracing::info!("listening on {}", addr);

        if let Err(err) = axum::Server::bind(&addr)
            .serve(routes.into_make_service())
            .with_graceful_shutdown(shutdown.listen())
            .await
        {
            tracing::error!("axum exited with error: {err}");
        }

        tracing::info!("Proper API shutdown.");
}
