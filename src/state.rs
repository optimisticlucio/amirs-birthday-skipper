use tokio::sync::broadcast;

use crate::player::{Player, PlayerId, PlayerList};
use crate::utils::Pronouns;
use serde::Serialize;

#[derive(Default, Clone)]
pub struct ServerState {
    pub active_game: Option<GameInfo>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Presentation {
    pub presenter_id: PlayerId,
    pub name: String,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: chrono::DateTime<chrono::Utc>,
}

impl Presentation {
    /// Returns a new presentation for the given user, setting the starting time to now.
    pub fn start_presentation_now(presenter: &Player) -> Self {
        Self {
            presenter_id: presenter.id,
            name: presenter.presentation_title.clone(),
            start_time: chrono::Utc::now(),
            end_time: chrono::Utc::now(), // Garbage value
        }
    }
}

#[derive(Clone)]
pub struct GameInfo {
    /// All the logged players in the current game, including the host.
    pub players: PlayerList,
    /// The ID of the player who is hosting the current game.
    pub host_id: PlayerId,
    /// A list of IDs for the players who haven't presented yet.
    pub players_who_havent_presented: Vec<PlayerId>,
    /// A list of all presentations complete until now, along with their data.
    pub complete_presentations: Vec<Presentation>,
    /// The title of the current game, for fun.
    pub session_name: String,
    /// What phase we're currently in.
    pub current_phase: GamePhase,
    /// Sends updates from other threads about when we should update the clients' presented data. Every websocket should be subscribed to this.
    pub broadcast_channel: broadcast::Sender<ServerMessage>,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub enum GamePhase {
    /// The game hasn't started yet and players are logging in.
    #[default]
    Setup,
    /// The host needs to select who will do the next presentation.
    SelectPresenter,
    /// The presentor is currently presenting their presentation.
    CurrentlyPresenting { current_presentation: Presentation },
    /// The game is fully over and we're letting people look at the results.
    Results,
}

#[derive(Debug, Clone, Serialize)]
/// Same as GamePhase, but for when we send data to the users. They don't need to bother with IDs.
pub enum PublicGamePhase {
    Setup {
        session_name: String,
        connected_players: Vec<Player>,
    },

    SelectPresenter {
        possible_presenters: Vec<Player>,
    },

    CurrentlyPresenting {
        presentation: Presentation,
    },

    Results {
        all_presentations: Vec<Presentation>,
    },
}

impl GameInfo {
    pub fn new(session_name: String, host_name: String, host_pronouns: Pronouns) -> Self {
        let mut players = PlayerList::default();
        let host = players.new_player(host_name, "The Host's Presentation".into(), host_pronouns);

        let (broadcast_channel, _reciever_channel) = broadcast::channel(16);

        Self {
            session_name,
            players,
            host_id: host.id,
            players_who_havent_presented: Vec::new(),
            complete_presentations: Vec::new(),
            current_phase: GamePhase::default(),
            broadcast_channel,
        }
    }

    /// Returns a list of all the players who still need to do a presentation.
    pub fn get_players_who_havent_presented(&self) -> Vec<Player> {
        self.players_who_havent_presented
            .iter()
            .map(|player_id| {
                self.players
                    .get_player_by_id(player_id)
                    .cloned()
                    .unwrap_or_default()
            })
            .collect()
    }

    pub fn get_public_game_phase(&self) -> PublicGamePhase {
        match &self.current_phase {
            GamePhase::Setup => PublicGamePhase::Setup {
                session_name: self.session_name.clone(),
                connected_players: self.players.to_vec(),
            },
            GamePhase::Results => PublicGamePhase::Results {
                all_presentations: self.complete_presentations.clone(),
            },
            GamePhase::CurrentlyPresenting {
                current_presentation,
            } => PublicGamePhase::CurrentlyPresenting {
                presentation: current_presentation.clone(),
            },
            GamePhase::SelectPresenter => PublicGamePhase::SelectPresenter {
                possible_presenters: self.get_players_who_havent_presented(),
            },
        }
    }

    // Does all the legwork to have the game be ready to play.
    pub fn initiate_game(&mut self) {
        self.players_who_havent_presented = self.players.get_all_ids();

        // Remove host from list of presentors
        self.players_who_havent_presented
            .retain(|&id| id != self.host_id);

        self.current_phase = GamePhase::SelectPresenter;
    }
}

#[derive(Clone, Serialize, Debug)]
pub enum ServerMessage {
    /// Notifies the client that we have changed the game state.
    SwitchPhase(PublicGamePhase),
    /// Not the user's fault; the message was invalid.
    InvalidJSON,
    /// The user's fault; the intent was invalid.
    InvalidRequest { reason: String },
    /// Not the user's fault, I fucked up somehow.
    InternalServerError { err: String },
}

impl GamePhase {
    /// Based on a given message from a user, return the new game state.
    fn act(&self, game_info: &mut GameInfo) {
        match self {
            Self::Setup => {
                todo!()
            }
            Self::SelectPresenter => {
                todo!()
            }
            Self::CurrentlyPresenting {
                current_presentation,
            } => {
                todo!()
            }
            Self::Results => {
                todo!()
            }
        }
    }
}
