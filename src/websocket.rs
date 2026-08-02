use std::ops::DerefMut;
use std::sync::Arc;
use tokio::sync::Mutex;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::{IntoResponse, Redirect};
use serde::{Deserialize, Serialize};
use tower_cookies::Cookies;

use crate::player::PlayerId;
use crate::state::{GameInfo, GamePhase, ServerMessage};
use crate::utils::Pronouns;
use crate::{Player, ServerState};

// WebSocketUpgrade: Extractor for establishing WebSocket connections.
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(server_state): State<Arc<Mutex<ServerState>>>,
    cookie_jar: Cookies,
) -> impl IntoResponse {
    // Before we start the websocket upgrade, check if there's a login cookie.
    let Some(this_user_id): Option<PlayerId> = cookie_jar
        .get("user_id")
        .and_then(|cookie| cookie.value().parse().ok())
    else {
        return Redirect::to("/login").into_response();
    };

    // Finalize upgrading the connection and call the provided callback with the stream.
    ws.on_failed_upgrade(|error| eprintln!("[WEBSOCKET] Error upgrading websocket: {}", error))
        .on_upgrade(move |socket| handle_user_socket(socket, server_state, this_user_id))
        .into_response()
}

/// Handles the messaging between a logged in user and the game. Assumes the user has already registered!
async fn handle_user_socket(
    mut socket: WebSocket,
    server_state_mutex: Arc<Mutex<ServerState>>,
    user_id: PlayerId,
) {
    // Returns `None` if the stream has closed.
    while let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            if let Message::Text(utf8_bytes) = msg {
                let user_message: UserMessage = match serde_json::from_str(&utf8_bytes) {
                    Ok(x) => x,
                    Err(err) => {
                        eprintln!(
                            "[WEBSOCKET] Failed reading JSON from user. Err: {err:?}\nJson is:\n{utf8_bytes}"
                        );
                        send_message_to_user(&mut socket, &ServerMessage::InvalidJSON).await;
                        break;
                    }
                };

                // Most user messages will be valid, so it's ok to get the mutex here already.
                // This is a birthday gift server, we're not dealing with large-scale DDOS attacks
                // where this shit matters.

                let mut server_state = server_state_mutex.lock().await;

                // If you entered this socket, I am hoping to GOD that the game is ongoing. If it isn't, what are you doing here.

                let Some(active_game) = server_state.active_game.as_mut() else {
                    // If the game isn't active, get out of this loop.
                    return;
                };

                match user_message {
                    UserMessage::StartGame => {
                        active_game.initiate_game();

                        // TODO: Broadcast to everyone that the game started.
                    }
                }
            }
        } else {
            let error = msg.err().unwrap();
            println!("Error receiving message: {:?}", error);

            break;
        }
    }
}

#[derive(Deserialize)]
/// Messages that the users send us throughout program runtime.
enum UserMessage {
    /// Sent by the host to start the game.
    StartGame,
}

async fn send_message_to_user(socket: &mut WebSocket, server_message: &ServerMessage) {
    if let Err(err) = socket
        .send(serde_json::to_string(server_message).unwrap().into())
        .await
    {
        eprintln!(
            "[WEBSOCKET] Failed sending message to user! Message: {server_message:?}, Err: {err:?}"
        );
    }
}
