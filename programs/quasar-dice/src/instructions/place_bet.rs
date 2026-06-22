//! A player bets against an open table. The table (and its commitment) must
//! already exist on-chain; the player adds a guess and their own `entropy`, and
//! stakes into the vault. The commitment is no longer a player argument: it was
//! posted by the house in `open_table`.

use {
    crate::{
        constants::*,
        error::DiceError,
        state::{Bet, BetInner, Table, VaultPda},
    },
    quasar_lang::{prelude::*, sysvars::Sysvar},
};

#[derive(Accounts)]
#[instruction(table_seed: u64)]
pub struct PlaceBet {
    #[account(mut)]
    pub player: Signer,

    /// The house identifies the vault and the table; it neither signs nor is
    /// mutated here.
    pub house: UncheckedAccount,

    #[account(mut, address = VaultPda::seeds(house.address()))]
    pub vault: UncheckedAccount,

    /// The open round. Loading it as `Account<Table>` proves it exists (the house
    /// has committed); a `place_bet` for an unopened table fails here.
    #[account(address = Table::seeds(house.address(), table_seed))]
    pub table: Account<Table>,

    #[account(
        init,
        payer = player,
        address = Bet::seeds(house.address(), table_seed, player.address())
    )]
    pub bet: Account<Bet>,

    pub system_program: Program<SystemProgram>,
}

impl PlaceBet {
    pub fn create_bet(
        &mut self,
        guess_roll: u8,
        amount: u64,
        entropy: [u8; 32],
        bumps: &PlaceBetBumps,
    ) -> Result<(), ProgramError> {
        require!(amount >= MIN_BET_LAMPORTS, DiceError::MinimumBet);
        require!(guess_roll >= MIN_ROLL, DiceError::MinimumRoll);
        require!(guess_roll <= MAX_ROLL, DiceError::MaximumRoll);

        let slot = Clock::get()?.slot.get();

        self.bet.set_inner(BetInner {
            player: *self.player.address(),
            amount,
            guess_roll,
            bump: bumps.bet,
            entropy,
            slot,
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
