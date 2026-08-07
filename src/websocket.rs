use std::ops::ControlFlow;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, broadcast, mpsc};

/// How long to let the writer task finish sending whatever is still queued after the
/// reader has given up on the connection.
const WRITER_DRAIN_GRACE: Duration = Duration::from_millis(250);

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::{IntoResponse, Redirect};
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use tower_cookies::Cookies;

use crate::ServerState;
use crate::player::PlayerId;
use crate::state::{GamePhase, Presentation, ServerMessage};

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
    socket: WebSocket,
    server_state_mutex: Arc<Mutex<ServerState>>,
    user_id: PlayerId,
) {
    // Subscribe *before* reading the phase, so that an update landing between the
    // two gets queued up for us rather than lost.
    let (broadcast_receiver, current_phase) = {
        let server_state = server_state_mutex.lock().await;

        // If you entered this socket, I am hoping to GOD that the game is ongoing. If it isn't, what are you doing here.
        let Some(active_game) = server_state.active_game.as_ref() else {
            return;
        };

        (
            active_game.broadcast_channel.subscribe(),
            active_game.get_public_game_phase(),
        )
    };

    // Split so that one task can sit waiting on the client while the other writes
    // to it. Past this point only the writer task ever touches the socket.
    let (mut socket_writer, socket_reader) = socket.split();

    // The broadcast channel reaches everyone, so anything meant for this user alone
    // (errors, mostly) needs its own lane to the writer task. Unbounded because that
    // makes sending synchronous, and we send while holding the state lock.
    let (personal_channel, personal_receiver) = mpsc::unbounded_channel();

    // A fresh client knows nothing, and the broadcast channel only carries messages
    // sent from this moment on. Catch them up before anything else.
    if !send_message_to_user(
        &mut socket_writer,
        &ServerMessage::SwitchPhase(current_phase),
    )
    .await
    {
        return;
    }

    let mut writer_task = tokio::spawn(handle_outgoing_messages(
        socket_writer,
        broadcast_receiver,
        personal_receiver,
    ));

    let mut reader_task = tokio::spawn(handle_incoming_messages(
        socket_reader,
        server_state_mutex,
        user_id,
        personal_channel,
    ));

    // Whichever half dies first takes the other one down with it.
    tokio::select! {
        // The writer is gone, so there is no way left to talk to this user anyway.
        _ = &mut writer_task => reader_task.abort(),

        // The reader dropping its end of the personal channel makes the writer send
        // off whatever is still queued and then stop by itself, so waiting on it here
        // gets a parting error message to the user instead of cutting it off. The
        // timeout is only in case the client stopped reading its end of the socket.
        _ = &mut reader_task => {
            if tokio::time::timeout(WRITER_DRAIN_GRACE, &mut writer_task)
                .await
                .is_err()
            {
                writer_task.abort();
            }
        }
    }
}

/// Forwards everything aimed at this user - broadcasts and personal messages alike -
/// onto their socket. Sole owner of the writing half of the socket.
async fn handle_outgoing_messages(
    mut socket_writer: SplitSink<WebSocket, Message>,
    mut broadcast_receiver: broadcast::Receiver<ServerMessage>,
    mut personal_receiver: mpsc::UnboundedReceiver<ServerMessage>,
) {
    loop {
        let message_to_send = tokio::select! {
            broadcast = broadcast_receiver.recv() => match broadcast {
                Ok(message) => message,
                // We only ever broadcast whole-state snapshots, so a client that fell
                // behind loses nothing by skipping straight to the newest one. This
                // stops being true the moment any broadcast becomes a delta.
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    eprintln!("[WEBSOCKET] User fell {skipped} messages behind, skipping them.");
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => break,
            },
            personal = personal_receiver.recv() => match personal {
                Some(message) => message,
                // The reader task is gone, so the connection is on its way out anyway.
                None => break,
            },
        };

        if !send_message_to_user(&mut socket_writer, &message_to_send).await {
            break;
        }
    }
}

/// Reads what the user sends us and acts on it. Sole owner of the reading half of the socket.
async fn handle_incoming_messages(
    mut socket_reader: SplitStream<WebSocket>,
    server_state_mutex: Arc<Mutex<ServerState>>,
    user_id: PlayerId,
    personal_channel: mpsc::UnboundedSender<ServerMessage>,
) {
    // Returns `None` if the stream has closed.
    while let Some(msg) = socket_reader.next().await {
        let msg = match msg {
            Ok(msg) => msg,
            Err(error) => {
                println!("Error receiving message: {:?}", error);
                break;
            }
        };

        let Message::Text(utf8_bytes) = msg else {
            continue;
        };

        let user_message: UserMessage = match serde_json::from_str(&utf8_bytes) {
            Ok(x) => x,
            Err(err) => {
                eprintln!(
                    "[WEBSOCKET] Failed reading JSON from user. Err: {err:?}\nJson is:\n{utf8_bytes}"
                );
                let _ = personal_channel.send(ServerMessage::InvalidJSON);
                break;
            }
        };

        if handle_user_message(
            user_message,
            &server_state_mutex,
            user_id,
            &personal_channel,
        )
        .await
        .is_break()
        {
            break;
        }
    }
}

/// Applies a single user message to the game, then tells everyone what the result was.
/// Breaking means this connection should be shut down.
async fn handle_user_message(
    user_message: UserMessage,
    server_state_mutex: &Arc<Mutex<ServerState>>,
    user_id: PlayerId,
    personal_channel: &mpsc::UnboundedSender<ServerMessage>,
) -> ControlFlow<()> {
    // Most user messages will be valid, so it's ok to get the mutex here already.
    // This is a birthday gift server, we're not dealing with large-scale DDOS attacks
    // where this shit matters.

    let mut server_state = server_state_mutex.lock().await;

    let Some(active_game) = server_state.active_game.as_mut() else {
        // If the game isn't active, get out of this loop.
        return ControlFlow::Break(());
    };

    match user_message {
        UserMessage::StartGame => {
            active_game.initiate_game();
        }
        UserMessage::SelectPresenter { user_id } => {
            if active_game.current_phase != GamePhase::SelectPresenter {
                let _ = personal_channel.send(ServerMessage::InvalidRequest {
                    reason: "Game is not in \"select presenter\" phase".to_string(),
                });
                return ControlFlow::Continue(());
            }

            let Some(presenting_player) = active_game.players.get_player_by_id(&user_id) else {
                let _ = personal_channel.send(ServerMessage::InternalServerError {
                    err: format!("GamePhase's UserId is invalid: ID is set to {user_id}"),
                });
                return ControlFlow::Continue(());
            };

            let new_presentation: Presentation =
                Presentation::start_presentation_now(presenting_player);

            active_game
                .players_who_havent_presented
                .retain(|&id| id != user_id);

            active_game.current_phase = GamePhase::CurrentlyPresenting {
                current_presentation: new_presentation,
            };
        }
        UserMessage::EndPresentation => {
            // If we're not currently presenting, this message is nonsense.
            let GamePhase::CurrentlyPresenting {
                ref mut current_presentation,
            } = active_game.current_phase
            else {
                let _ = personal_channel.send(ServerMessage::InvalidRequest {
                    reason: "Cannot end presentation when none are active.".to_string(),
                });
                return ControlFlow::Continue(());
            };

            // If it's not the host or presenter, gtfo.
            if !(user_id == active_game.host_id || user_id == current_presentation.presenter_id) {
                let _ = personal_channel.send(ServerMessage::InvalidRequest {
                    reason: "Non-host user cannot end someone else's presentation".to_string(),
                });
                return ControlFlow::Continue(());
            }

            let GamePhase::CurrentlyPresenting {
                mut current_presentation,
            } = std::mem::replace(&mut active_game.current_phase, GamePhase::SelectPresenter)
            else {
                unreachable!(
                    "We already determined above that current_phase is CurrentlyPresenting."
                );
            };

            current_presentation.end_time = chrono::Utc::now();

            active_game
                .complete_presentations
                .push(current_presentation);
        }
    }

    // Every branch above moved the game along, so tell everyone where we are now.
    // This only errors when nobody is subscribed, which is not our problem.
    let new_phase = active_game.get_public_game_phase();
    let _ = active_game
        .broadcast_channel
        .send(ServerMessage::SwitchPhase(new_phase));

    ControlFlow::Continue(())
}

#[derive(Deserialize)]
/// Messages that the users send us throughout program runtime.
enum UserMessage {
    /// Sent by the host to start the game.
    StartGame,
    /// Sent by the host to indicate which player should present next.
    SelectPresenter { user_id: PlayerId },
    /// Sent either by the host or the presentor to end the presentation time.
    EndPresentation,
}

/// Writes a single message out to a user. Returns whether the socket is still usable.
async fn send_message_to_user(
    socket_writer: &mut SplitSink<WebSocket, Message>,
    server_message: &ServerMessage,
) -> bool {
    if let Err(err) = socket_writer
        .send(serde_json::to_string(server_message).unwrap().into())
        .await
    {
        eprintln!(
            "[WEBSOCKET] Failed sending message to user! Message: {server_message:?}, Err: {err:?}"
        );
        return false;
    }

    true
}
