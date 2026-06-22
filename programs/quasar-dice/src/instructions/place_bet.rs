//! Player commits a bet and deposits the stake. The Bet PDA records the guess
//! and the `sha256(preimage)` commitment; the stake moves into the vault.

use {
    crate::{constants::*, error::DiceError, state::{Bet, BetInner, VaultPda}},
    quasar_lang::{prelude::*, sysvars::Sysvar},
};

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct PlaceBet {
    #[account(mut)]
    pub player: Signer,

    /// The house identifies the vault; it neither signs nor is mutated here.
    pub house: UncheckedAccount,

    #[account(mut, address = VaultPda::seeds(house.address()))]
    pub vault: UncheckedAccount,

    #[account(
        init,
        payer = player,
        address = Bet::seeds(vault.address(), player.address(), seed)
    )]
    pub bet: Account<Bet>,

    pub system_program: Program<SystemProgram>,
}

impl PlaceBet {
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub fn create_bet(
        &mut self,
        seed: u64,
        guess_roll: u8,
        amount: u64,
        commitment: [u8; 32],
        entropy: [u8; 32],
        bumps: &PlaceBetBumps,
    ) -> Result<(), ProgramError> {
        require!(amount >= MIN_BET_LAMPORTS, DiceError::MinimumBet);
        require!(guess_roll >= MIN_ROLL, DiceError::MinimumRoll);
        require!(guess_roll <= MAX_ROLL, DiceError::MaximumRoll);

        let slot = Clock::get()?.slot.get();

        self.bet.set_inner(BetInner {
            player: *self.player.address(),
            seed,
            slot,
            amount,
            guess_roll,
            bump: bumps.bet,
            commitment,
            entropy,
        });
        Ok(())
    }

    #[inline(always)]
    pub fn deposit(&self, amount: u64) -> Result<(), ProgramError> {
        self.system_program
            .transfer(&self.player, &self.vault, amount)
            .invoke()
    }
}
