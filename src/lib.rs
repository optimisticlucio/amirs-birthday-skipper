use axum::Router;
pub use player::Player;
pub use state::ServerState;
use tower_http::services::ServeDir;

pub mod player;
pub mod state;
pub mod utils;

pub fn router(state: ServerState) -> Router<()> {
    Router::new()
        // TODO: Add ws handling on /websocket path
        .fallback_service(axum::routing::get_service(ServeDir::new("./site_data")))
        .with_state(state)
}
