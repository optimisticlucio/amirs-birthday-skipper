use rand::RngExt;
use serde::{Deserialize, Serialize};

use crate::utils::Pronouns;
use std::collections::HashMap;

pub type PlayerId = u16; // We got like, 25 players tops. This is beyond overkill as is.

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Represents an individual, logged-in player in the current game.
pub struct Player {
    #[serde(default)]
    pub id: PlayerId,
    pub name: String,
    pub presentation_title: String,
    pub pronouns: Pronouns,
}

impl PartialEq for Player {
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id)
    }
}

impl Default for Player {
    fn default() -> Self {
        Player {
            id: 0,
            name: "Default Player".into(),
            presentation_title: "The Wonders of Debugging".into(),
            pronouns: Pronouns::Mixed,
        }
    }
}

#[derive(Debug, Default, Clone)]
/// Represents all the players currently in the game. Mostly just a list with extra assisting functions.
pub struct PlayerList {
    // The index should be the same type as the ID
    players: HashMap<u16, Player>,
}

impl PlayerList {
    /// Adds a new player to the list, with a given name, presentation name, and pronoun set.
    pub fn new_player(
        &mut self,
        name: String,
        presentation_title: String,
        pronouns: Pronouns,
    ) -> Player {
        let mut id;

        // Get an ID that isn't in the list yet.
        loop {
            id = rand::rng().random();
            if self.get_player_by_id(&id).is_none() {
                break;
            }
        }

        let new_player = Player {
            id,
            name,
            presentation_title,
            pronouns,
        };

        self.players.insert(id, new_player.clone());

        new_player
    }

    /// Returns a player by an ID if they exist. If they do not, returns None.
    pub fn get_player_by_id(&self, id: &PlayerId) -> Option<&Player> {
        self.players.get(id)
    }

    /// Get a vec of all players in the game.
    pub fn into_vec(&self) -> Vec<Player> {
        self.players.values().cloned().collect()
    }

    pub fn get_all_ids(&self) -> Vec<PlayerId> {
        self.players.keys().cloned().collect()
    }
}
