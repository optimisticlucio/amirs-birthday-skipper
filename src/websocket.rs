use axum::extract::WebSocketUpgrade;
use axum::extract::ws::{WebSocket, Message};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};


// WebSocketUpgrade: Extractor for establishing WebSocket connections.
pub async fn websocket_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    // Finalize upgrading the connection and call the provided callback with the stream.
    ws.on_failed_upgrade(|error| eprintln!("[WEBSOCKET] Error upgrading websocket: {}", error))
        .on_upgrade(handle_socket)
}

// WebSocket: A stream of WebSocket messages.
async fn handle_socket(mut socket: WebSocket) {
    // Returns `None` if the stream has closed.
    while let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            if let Message::Text(utf8_bytes) = msg {
                let user_message: UserMessage = match serde_json::from_str(&utf8_bytes) {
                    Ok(x) => x, 
                    Err(err) => {
                            eprintln!("[WEBSOCKET] Failed reading JSON from user. Err: {err:?}\nJson is:\n{utf8_bytes}");
                        if let Err(err) = socket.send(serde_json::to_string(&ServerMessage::InvalidJSON).unwrap().into()).await {
                            eprintln!("[WEBSOCKET] Failed sending Invalid JSON message to user! {err:?}");
                        }
                        break;
                    }
                };

                match user_message {
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

}

#[derive(Serialize)] 
/// Messages that the server sends to the user 
enum ServerMessage {
    /// Not the user's fault; the message was invalid.
    InvalidJSON
}

