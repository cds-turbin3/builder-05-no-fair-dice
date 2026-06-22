//! Commit-reveal dice game, a native Quasar program. Pure SOL custody (a
//! house-owned vault PDA), an on-chain commit (the house opens a table), commit-
//! reveal randomness over sha256, and instruction introspection: `resolve_bet`
//! reads the preceding `reveal` instruction out of the Instructions sysvar to
//! recover the preimage and confirm the house signed it.
//!
//! Discriminators: initialize 0, place_bet 1, reveal 2, resolve_bet 3,
//! refund_bet 4, open_table 5, close_table 6. `reveal`'s discriminator (2) is
//! what the introspection parse checks; keep it in sync with `REVEAL_DISCRIMINATOR`.

#![no_std]
// Accounts-struct fields are consumed by the derive macro's generated init / CPI
// code, not always by the handler, so rustc sees some as unread.
#![allow(dead_code)]

use quasar_lang::prelude::*;

mod constants;
mod error;
mod instructions;
mod introspection;
mod state;

use instructions::*;

declare_id!("8hjbFtnfY87ZzpEpx26u5tx4KjxkrqiWGUEcWFbDNn7h");

/// Discriminator of the `reveal` instruction, as seen by the introspection parse.
pub const REVEAL_DISCRIMINATOR: u8 = 2;

/// sha256 via the `sol_sha256` syscall. Declared directly (no crate dependency)
/// to avoid version-coordinating a hasher against Quasar's solana graph. On a
/// non-SBF host build the program logic never runs, so the body is a stub.
#[cfg(target_os = "solana")]
pub(crate) fn sha256(input: &[u8]) -> [u8; 32] {
    extern "C" {
        fn sol_sha256(vals: *const u8, val_len: u64, hash_result: *mut u8) -> u64;
    }
    let slices: [&[u8]; 1] = [input];
    let mut hash = core::mem::MaybeUninit::<[u8; 32]>::uninit();
    // SAFETY: on SBF `&[u8]` has layout `(*const u8, u64)` == `SolBytes`, so the
    // slice array passes directly; `sol_sha256` fills all 32 bytes.
    unsafe {
        sol_sha256(
            slices.as_ptr() as *const u8,
            slices.len() as u64,
            hash.as_mut_ptr() as *mut u8,
        );
        hash.assume_init()
    }
}

#[cfg(not(target_os = "solana"))]
pub(crate) fn sha256(_input: &[u8]) -> [u8; 32] {
    [0u8; 32]
}

#[program]
mod quasar_dice {
    use super::*;

    /// House seeds its vault with an initial bankroll.
    #[instruction(discriminator = 0)]
    pub fn initialize(ctx: Ctx<Initialize>, amount: u64) -> Result<(), ProgramError> {
        ctx.accounts.handle(amount)
    }

    /// House opens a round by posting its `sha256(preimage)` commitment on-chain,
    /// before any bet. `table_seed` identifies the Table PDA.
    #[instruction(discriminator = 5)]
    pub fn open_table(
        ctx: Ctx<OpenTable>,
        table_seed: u64,
        commitment: [u8; 32],
    ) -> Result<(), ProgramError> {
        let _ = table_seed;
        ctx.accounts.handle(commitment, &ctx.bumps)
    }

    /// Player bets against an open table: records the guess and the player's own
    /// `entropy`, and deposits the stake. `table_seed` identifies the round.
    #[instruction(discriminator = 1)]
    pub fn place_bet(
        ctx: Ctx<PlaceBet>,
        table_seed: u64,
        amount: u64,
        guess_roll: u8,
        entropy: [u8; 32],
    ) -> Result<(), ProgramError> {
        let _ = table_seed;
        ctx.accounts
            .create_bet(guess_roll, amount, entropy, &ctx.bumps)?;
        ctx.accounts.deposit(amount)
    }

    /// Marker instruction: carries the preimage and the house signature that
    /// `resolve_bet` introspects. The handler itself does nothing.
    #[instruction(discriminator = 2)]
    pub fn reveal(ctx: Ctx<Reveal>, preimage: [u8; 32]) -> Result<(), ProgramError> {
        let _ = preimage;
        ctx.accounts.handle()
    }

    /// House settles a bet. Must directly follow a `reveal` in the same
    /// transaction; the preimage is recovered by introspection. `table_seed`
    /// identifies the round. The bet closes to the player, the table to the house.
    #[instruction(discriminator = 3)]
    pub fn resolve_bet(ctx: Ctx<ResolveBet>, table_seed: u64) -> Result<(), ProgramError> {
        let _ = table_seed;
        ctx.accounts.resolve(&ctx.bumps)
    }

    /// Player reclaims a stale, unrevealed bet after the timeout. `table_seed`
    /// identifies the round. The bet and the abandoned table both close.
    #[instruction(discriminator = 4)]
    pub fn refund_bet(ctx: Ctx<RefundBet>, table_seed: u64) -> Result<(), ProgramError> {
        let _ = table_seed;
        ctx.accounts.refund(&ctx.bumps)
    }

    /// House reclaims a table it opened but that no one bet against. A claimed
    /// (bet-against) table is rejected; it is retired by settle or refund instead.
    #[instruction(discriminator = 6)]
    pub fn close_table(ctx: Ctx<CloseTable>, table_seed: u64) -> Result<(), ProgramError> {
        let _ = table_seed;
        ctx.accounts.handle()
    }
}
