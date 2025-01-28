mod inputs;

use axum::Server;
use inputs::http::create_http_router;
use std::net::SocketAddr;
use tracing::info;
use tracing_subscriber;

#[tokio::main]
async fn main() {
    // Set up tracing for logging
    tracing_subscriber::fmt().init();

    // Define the HTTP server address
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("HTTP log collector running at http://{}", addr);

    // Create the HTTP router
    let app = create_http_router();

    // Start the HTTP server
    Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
