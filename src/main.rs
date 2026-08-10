use std::env;

use amirs_birthday_skipper::ServerState;

#[tokio::main]
async fn main() {
    println!("[STARTUP] Server is initializing...");

    let state = ServerState::default();

    let app = amirs_birthday_skipper::router(state);

    let server_port = env::var("SERVER_PORT").unwrap_or("8080".to_string());

    let listener = tokio::net::TcpListener::bind(format!("[::]:{server_port}"))
        .await
        .unwrap_or_else(|_| panic!("failed to bind port {server_port}"));

    println!("[STARTUP] Server successfully initialized! Starting server.");

    axum::serve(listener, app.into_make_service())
        .await
        .unwrap()
}
