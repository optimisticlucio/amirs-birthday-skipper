use axum::{
    Router,
    routing::{get, post},
};
pub use player::Player;
pub use state::ServerState;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::services::ServeDir;

pub mod login;
pub mod player;
pub mod state;
pub mod utils;
pub mod websocket;

pub fn router(state: ServerState) -> Router<()> {
    Router::new()
        .route("/websocket", get(websocket::websocket_handler))
        .route("/login", post(login::login_logic_handler))
        .fallback_service(axum::routing::get_service(ServeDir::new("./site_data")))
        .with_state(Arc::new(Mutex::new(state)))
}
