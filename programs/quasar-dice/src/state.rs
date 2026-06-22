//! Persistent on-chain state and the PDA seed markers.

use quasar_lang::prelude::*;

/// A single bet. PDA: `[b"bet", vault, player, seed]`; `seed` lets one player
/// hold several concurrent bets against the same vault.
///
/// `commitment` is `sha256(preimage)`, the house's hidden randomness; the
/// preimage is a fixed 32 bytes (commit-reveal's canonical form), which keeps the
/// `reveal` instruction's wire layout fixed so the introspection parse is
/// byte-exact rather than dependent on Quasar's variable-length argument encoding.
///
/// `entropy` is the player's own 32 bytes, contributed in the open at bet time.
/// The roll is `sha256(preimage ++ entropy)`, so neither side fixes it alone: the
/// house commits the preimage before seeing `entropy`, and the player cannot
/// derive the roll from the commitment without the preimage it hides.
#[account(discriminator = 1, set_inner)]
#[seeds(b"bet", vault: Address, player: Address, seed: u64)]
pub struct Bet {
    pub player: Address,
    pub seed: u64,
    pub slot: u64,
    pub amount: u64,
    pub guess_roll: u8,
    pub bump: u8,
    pub commitment: [u8; 32],
    pub entropy: [u8; 32],
}

/// Seed marker for the house's SOL vault: `[b"vault", house]`. A `#[derive(Seeds)]`
/// marker (not an account) so the vault can be a program-addressed PDA that signs
/// payouts, rather than a caller-provided keypair.
#[derive(Seeds)]
#[seeds(b"vault", house: Address)]
pub struct VaultPda;
