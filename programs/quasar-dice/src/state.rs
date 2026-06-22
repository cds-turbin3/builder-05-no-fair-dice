//! Persistent on-chain state and the PDA seed markers.

use quasar_lang::prelude::*;

/// A round the house has opened. PDA: `[b"table", house, table_seed]`. The house
/// posts its `commitment = sha256(preimage)` here, signed, BEFORE any bet — so
/// the chain witnesses the commit and the player reads it from state rather than
/// trusting an off-chain handoff.
#[account(discriminator = 2, set_inner)]
#[seeds(b"table", house: Address, table_seed: u64)]
pub struct Table {
    pub house: Address,
    pub commitment: [u8; 32],
    /// Set true by the first (and only) bet. A claimed table can no longer be
    /// closed by `close_table`, so the house cannot close-and-reopen it with a
    /// substituted commitment after seeing the player's entropy.
    pub claimed: bool,
    pub bump: u8,
}

/// A player's bet against an open table. PDA: `[b"bet", house, table_seed, player]`.
///
/// The roll is `sha256(preimage ++ entropy)`: the house's preimage (hidden behind
/// the table's commitment) mixed with the player's own `entropy`, contributed in
/// the open here, so neither side fixes the outcome alone.
#[account(discriminator = 1, set_inner)]
#[seeds(b"bet", house: Address, table_seed: u64, player: Address)]
pub struct Bet {
    pub player: Address,
    pub amount: u64,
    pub guess_roll: u8,
    pub bump: u8,
    pub entropy: [u8; 32],
    pub slot: u64,
}

/// Seed marker for the house's SOL vault: `[b"vault", house]`. A `#[derive(Seeds)]`
/// marker (not an account) so the vault is a program-addressed PDA that signs
/// payouts, rather than a caller-provided keypair.
#[derive(Seeds)]
#[seeds(b"vault", house: Address)]
pub struct VaultPda;
