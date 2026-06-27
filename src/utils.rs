use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
/// The supported sets of pronouns in this game.
pub enum Pronouns {
    Male,
    Female,
    Mixed,
}
