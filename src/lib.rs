use axum::{Router, routing::get};
pub use player::Player;
pub use state::ServerState;
use tower_http::services::ServeDir;

pub mod player;
pub mod state;
pub mod utils;
pub mod websocket;

pub fn router(state: ServerState) -> Router<()> {
    Router::new()
        .route_service("/websocket", get(websocket::websocket_handler))
        .fallback_service(axum::routing::get_service(ServeDir::new("./site_data")))
        .with_state(state)
}
