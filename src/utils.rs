use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
/// The supported sets of pronouns in this game.
pub enum Pronouns {
    Male,
    Female,
    Mixed,
}
