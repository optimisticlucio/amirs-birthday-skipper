use axum::extract::{State, Query};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::{Player, ServerState};
use axum::response::IntoResponse;


/// Handles the UI of users logging into the game, and giving them a session cookie.
pub async fn login_ui_handler(
    State(server_state): State<Arc<Mutex<ServerState>>>,
) -> impl IntoResponse {
    // TODO: Implement login screen.
}


#[axum::debug_handler]
/// Handles recieving the user data for logging in users, and giving them back the relevant cookie to log in with.
pub async fn login_logic_handler(
    State(server_state): State<Arc<Mutex<ServerState>>>,
    Query(query): Query<Player>,
) -> impl IntoResponse {
    // TODO: Implement login screen.
}

