use tokio::sync::broadcast;

use crate::{Player, player::PlayerList};
use serde::Serialize;

#[derive(Default, Clone)]
pub struct ServerState {
    active_game: Option<GameInfo>,
}

#[derive(Clone)]
struct GameInfo {
    /// All the logged players in the current game, including the host.
    pub players: PlayerList,
    /// The player who is hosting the current game.
    pub host: Player,
    /// A list of players who haven't presented yet.
    pub players_who_havent_presented: Vec<Player>,
    /// The title of the current game, for fun.
    pub session_name: String,
    /// What phase we're currently in.
    pub current_phase: GamePhase,
    /// Sends updates from other threads about when we should update the clients' presented data. Every websocket should be subscribed to this.
    pub broadcast_channel: broadcast::Sender<BroadcastMessage>,
}

#[derive(Debug, Clone, Default, Serialize)]
enum GamePhase {
    /// The game hasn't started yet and players are logging in.
    #[default]
    Setup,
    /// The game is fully over and we're letting people look at the results.
    Results,
    /// The host needs to select who will do the next presentation.
    SelectPresentor,
    /// The presentor is currently presenting their presentation.
    CurrentlyPresenting {
        presentor: Player,
        presentation_start_time: chrono::DateTime<chrono::Utc>,
    },
    /// The presentor has finished presenting and people are voting on their performance.
    PostPresenting { presentor: Player },
}

impl GameInfo {
    fn new(session_name: String, host: Player) -> Self {
        let players = PlayerList::default();

        let (broadcast_channel, _reciever_channel) = broadcast::channel(16);

        Self {
            session_name,
            players,
            host,
            players_who_havent_presented: Vec::new(),
            current_phase: GamePhase::default(),
            broadcast_channel,
        }
    }
}

#[derive(Clone, Serialize)]
enum BroadcastMessage {
    /// Notifies the client that we have changed the game state.
    SwitchPhase(GamePhase),
}
