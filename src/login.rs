use crate::player::PlayerId;
use crate::state::GameInfo;
use crate::{Player, ServerState};
use axum::extract::{Query, State};
use axum::response::{IntoResponse, Redirect};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_cookies::{Cookie, Cookies};

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
    Query(mut passed_user): Query<Player>,
    cookie_jar: Cookies,
) -> impl IntoResponse {
    // TODO: Correct possibly invalid data passed by user.

    // TODO: Verify that the user data passed makes sense/is legal.

    let mut unlocked_server_state = server_state.lock().await;

    let user_id: PlayerId;

    if let Some(active_game) = &mut unlocked_server_state.active_game {
        // We're adding a player.
        let new_user = active_game.players.new_player(
            passed_user.name,
            passed_user.presentation_title,
            passed_user.pronouns,
        );

        user_id = new_user.id;
    } else {
        // We're adding the host!

        // The "presentation name" is the session name. It's hacky but fuck it, no one is gonna be maintaining this.
        // If for whatever reason you're maintaining this and reusing this code, dm me and I'll buy you a pizza for the trouble.
        let new_game = GameInfo::new(
            passed_user.presentation_title,
            passed_user.name,
            passed_user.pronouns,
        );

        user_id = new_game.host_id;

        unlocked_server_state.active_game = Some(new_game);
    }

    // Give them the cookie and send them to play the game.

    let host_cookie = Cookie::new("user_id", user_id.to_string());

    cookie_jar.add(host_cookie);

    Redirect::to("/")
}
