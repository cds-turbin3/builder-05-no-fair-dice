//! Program error codes. Quasar maps each `#[error_code]` variant to
//! `ProgramError::Custom(ordinal)` (so `CommitRevealMismatch` surfaces as `0x4`).

use quasar_lang::prelude::*;

#[error_code]
pub enum DiceError {
    MinimumBet,
    MinimumRoll,
    MaximumRoll,
    BadSignature,
    CommitRevealMismatch,
    Overflow,
    TimeoutNotReached,
    /// The introspected previous instruction was not this program's `reveal`.
    NotReveal,
}
