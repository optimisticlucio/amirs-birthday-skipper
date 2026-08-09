use crate::player::PlayerId;
use crate::state::GameInfo;
use crate::{Player, ServerState};
use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Redirect};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_cookies::{Cookie, Cookies};

#[axum::debug_handler]
/// Handles recieving the user data for logging in users, and giving them back the relevant cookie to log in with.
pub async fn login_logic_handler(
    State(server_state): State<Arc<Mutex<ServerState>>>,
    cookie_jar: Cookies,
    Json(mut passed_user): Json<Player>,
) -> impl IntoResponse {
    sanitize_player(&mut passed_user);

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
            passed_user.name,
            passed_user.pronouns,
            passed_user.presentation_title,
        );

        user_id = new_game.host_id;

        unlocked_server_state.active_game = Some(new_game);
    }

    // Give them the cookie and send them to play the game.

    let host_cookie = Cookie::new("user_id", user_id.to_string());

    cookie_jar.add(host_cookie);

    Redirect::to("/")
}

/// Takes a passed user, and cleans them of any invalid or just off data.
fn sanitize_player(player: &mut Player) {
    player.name = player.name.trim().to_owned();

    player.presentation_title = player
        .presentation_title
        .as_ref()
        .map(|title| title.trim().to_owned())
        .filter(|title| !title.is_empty());
}
