use std::collections::HashSet;

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
    /// Returns a new presentation for the given user, setting the starting time to now. If the user does not have a presentation, returns None.
    pub fn start_presentation_now(presenter: &Player) -> Option<Self> {
        let Some(user_presentation_name) = &presenter.presentation_title else {
            return None;
        };

        Some(Self {
            presenter_id: presenter.id,
            name: user_presentation_name.clone(),
            start_time: chrono::Utc::now(),
            end_time: chrono::Utc::now(), // Garbage value
        })
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
    /// What phase we're currently in.
    pub current_phase: GamePhase,
    /// Sends updates from other threads about when we should update the clients' presented data. Every websocket should be subscribed to this.
    pub broadcast_channel: broadcast::Sender<ServerMessage>,
    /// The percentage, set by the host, of how many of the present players need to press skip to skip the presentation.
    pub skip_percentage: f64,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub enum GamePhase {
    /// The game hasn't started yet and players are logging in.
    #[default]
    Setup,
    /// The host needs to select who will do the next presentation.
    SelectPresenter,
    /// The presentor is currently presenting their presentation.
    CurrentlyPresenting {
        current_presentation: Presentation,
        users_who_voted_to_skip: HashSet<PlayerId>,
    },
    /// The game is fully over and we're letting people look at the results.
    Results,
}

#[derive(Debug, Clone, Serialize)]
/// Same as GamePhase, but for when we send data to the users. They don't need to bother with IDs.
pub enum PublicGamePhase {
    Setup {
        connected_players: Vec<Player>,
    },

    SelectPresenter {
        possible_presenters: Vec<Player>,
    },

    CurrentlyPresenting {
        presentation: Presentation,
        players_who_skipped: Vec<Player>,
        total_players: usize,
    },

    Results {
        all_presentations: Vec<Presentation>,
    },
}

impl GameInfo {
    pub fn new(
        host_name: String,
        host_pronouns: Pronouns,
        host_presentation: Option<String>,
    ) -> Self {
        let mut players = PlayerList::default();
        let host = players.new_player(host_name, host_presentation, host_pronouns);

        let (broadcast_channel, _reciever_channel) = broadcast::channel(16);

        Self {
            players,
            host_id: host.id,
            players_who_havent_presented: Vec::new(),
            complete_presentations: Vec::new(),
            current_phase: GamePhase::default(),
            broadcast_channel,
            // The default value of the skip percentage, according to Amir, should be 75%.
            skip_percentage: 0.75,
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
                connected_players: self.players.to_vec(),
            },
            GamePhase::Results => PublicGamePhase::Results {
                all_presentations: self.complete_presentations.clone(),
            },
            GamePhase::CurrentlyPresenting {
                current_presentation,
                users_who_voted_to_skip,
            } => PublicGamePhase::CurrentlyPresenting {
                presentation: current_presentation.clone(),
                total_players: self.players.size(),
                players_who_skipped: users_who_voted_to_skip
                    .iter()
                    .map(|player_id| self.players.get_player_by_id(player_id).unwrap().to_owned())
                    .collect(),
            },
            GamePhase::SelectPresenter => PublicGamePhase::SelectPresenter {
                possible_presenters: self.get_players_who_havent_presented(),
            },
        }
    }

    // Does all the legwork to have the game be ready to play.
    pub fn initiate_game(&mut self) {
        // Only collect player ids of players with a presentation title.
        self.players_who_havent_presented = self
            .players
            .to_vec()
            .iter()
            .filter(|player| player.presentation_title.is_some())
            .map(|player| player.id)
            .collect();

        self.current_phase = GamePhase::SelectPresenter;
    }

    // Ends the current presentation and switches to the appropriate state afterwards. Returns false if the current phase wasn't CurrentlyPresenting and the conversion failed.
    pub fn end_presentation(&mut self) -> bool {
        // Is the game over yet?
        let next_phase: GamePhase = {
            if self.get_players_who_havent_presented().is_empty() {
                GamePhase::Results
            } else {
                GamePhase::SelectPresenter
            }
        };

        let GamePhase::CurrentlyPresenting {
            mut current_presentation,
            ..
        } = std::mem::replace(&mut self.current_phase, next_phase)
        else {
            return false;
        };

        current_presentation.end_time = chrono::Utc::now();

        self.complete_presentations.push(current_presentation);

        true
    }

    // Checks whether enough players have voted to skip the current presentation, and if so, skips. Returns false if the current phase wasn't CurrentlyPresenting and so this is a nonsense check.
    pub fn check_for_skip_percentage(&mut self) -> bool {
        let GamePhase::CurrentlyPresenting {
            users_who_voted_to_skip,
            ..
        } = &self.current_phase
        else {
            return false;
        };

        let percentage_who_voted_to_skip: f64 =
            users_who_voted_to_skip.len() as f64 / self.players.size() as f64;

        if percentage_who_voted_to_skip >= self.skip_percentage {
            self.end_presentation();
        }

        true
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
