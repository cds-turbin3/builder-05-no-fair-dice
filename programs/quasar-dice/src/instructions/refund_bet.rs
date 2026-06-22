//! The player reclaims a stale bet the house never revealed. After
//! `REFUND_TIMEOUT_SLOTS`, the stake returns from the vault and the Bet closes.
//! The table is left for the house to settle or clean up.

use {
    crate::{
        constants::REFUND_TIMEOUT_SLOTS,
        error::DiceError,
        state::{Bet, VaultPda},
    },
    quasar_lang::{cpi::Seed, prelude::*, sysvars::Sysvar},
};

#[derive(Accounts)]
#[instruction(table_seed: u64)]
pub struct RefundBet {
    #[account(mut)]
    pub player: Signer,

    pub house: UncheckedAccount,

    #[account(mut, address = VaultPda::seeds(house.address()))]
    pub vault: UncheckedAccount,

    #[account(
        mut,
        has_one(player),
        close(dest = player),
        address = Bet::seeds(house.address(), table_seed, player.address())
    )]
    pub bet: Account<Bet>,

    pub system_program: Program<SystemProgram>,
}

impl RefundBet {
    pub fn refund(&mut self, bumps: &RefundBetBumps) -> Result<(), ProgramError> {
        let slot = Clock::get()?.slot.get();
        let elapsed = slot
            .checked_sub(self.bet.slot.get())
            .ok_or(DiceError::Overflow)?;
        require!(elapsed > REFUND_TIMEOUT_SLOTS, DiceError::TimeoutNotReached);

        let amount = self.bet.amount.get();
        let house = *self.house.address();
        let bump = [bumps.vault];
        let seeds = [
            Seed::from(b"vault" as &[u8]),
            Seed::from(house.as_ref()),
            Seed::from(bump.as_ref()),
        ];
        self.system_program
            .transfer(&self.vault, &self.player, amount)
            .invoke_signed(&seeds)?;
        Ok(())
    }
}
