use std::ops::DerefMut;
use std::sync::Arc;
use tokio::sync::Mutex;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

use crate::state::{GameInfo, GamePhase, ServerMessage};
use crate::utils::Pronouns;
use crate::{Player, ServerState};

// WebSocketUpgrade: Extractor for establishing WebSocket connections.
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(server_state): State<Arc<Mutex<ServerState>>>,
) -> impl IntoResponse {
    // Finalize upgrading the connection and call the provided callback with the stream.
    ws.on_failed_upgrade(|error| eprintln!("[WEBSOCKET] Error upgrading websocket: {}", error))
        .on_upgrade(move |socket| handle_socket(socket, server_state))
}

// WebSocket: A stream of WebSocket messages.
async fn handle_socket(mut socket: WebSocket, server_state_mutex: Arc<Mutex<ServerState>>) {
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

                let mut server_state_lock = server_state_mutex.lock().await;

                let server_state = server_state_lock.deref_mut();

                match user_message {
                    UserMessage::InitializeGame {
                        host_name,
                        host_pronouns,
                        session_name,
                    } => {
                        // If the game was already initialized, fuck you doin here?
                        if server_state.active_game.is_some() {
                            send_message_to_user(
                                &mut socket,
                                &ServerMessage::InvalidRequest {
                                    reason: "Cannot initialize game while one is already running."
                                        .to_string(),
                                },
                            )
                            .await;
                            break;
                        }

                        let host: Player = crate::Player {
                            id: 0,
                            name: host_name,
                            presentation_title: "Host Isn't Presenting".to_string(),
                            pronouns: host_pronouns,
                        };

                        let game_info: GameInfo = GameInfo::new(session_name, host);

                        server_state.active_game = Some(game_info);

                        // TODO: Update user that the game started.
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
    /// Message sent by the first person to connect to the client, setting up the basic fundamentals of the session.
    InitializeGame {
        session_name: String,
        host_name: String,
        host_pronouns: Pronouns,
    },
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
