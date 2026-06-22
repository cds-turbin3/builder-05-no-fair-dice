//! The house reclaims a table it opened but that no one bet against. Guarded by
//! the `claimed` flag: a table with a bet against it cannot be closed here, so
//! the house cannot close-and-reopen a live round to substitute its commitment
//! after seeing the player's entropy. A claimed table is only retired by
//! `resolve_bet` (settled) or `refund_bet` (abandoned).

use {
    crate::{error::DiceError, state::Table},
    quasar_lang::prelude::*,
};

#[derive(Accounts)]
#[instruction(table_seed: u64)]
pub struct CloseTable {
    #[account(mut)]
    pub house: Signer,

    #[account(
        mut,
        has_one(house),
        close(dest = house),
        address = Table::seeds(house.address(), table_seed)
    )]
    pub table: Account<Table>,

    pub system_program: Program<SystemProgram>,
}

impl CloseTable {
    #[inline(always)]
    pub fn handle(&self) -> Result<(), ProgramError> {
        let claimed: bool = self.table.claimed.into();
        require!(!claimed, DiceError::TableInUse);
        Ok(())
    }
}
