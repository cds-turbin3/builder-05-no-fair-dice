//! The house opens a round by posting its commitment on-chain. House-signed, and
//! it must land before any bet, so the chain itself witnesses that the house
//! committed to a preimage before the player chose entropy and staked.

use {
    crate::state::{Table, TableInner},
    quasar_lang::prelude::*,
};

#[derive(Accounts)]
#[instruction(table_seed: u64)]
pub struct OpenTable {
    #[account(mut)]
    pub house: Signer,

    #[account(init, payer = house, address = Table::seeds(house.address(), table_seed))]
    pub table: Account<Table>,

    pub system_program: Program<SystemProgram>,
}

impl OpenTable {
    #[inline(always)]
    pub fn handle(&mut self, commitment: [u8; 32], bumps: &OpenTableBumps) -> Result<(), ProgramError> {
        self.table.set_inner(TableInner {
            house: *self.house.address(),
            commitment,
            bump: bumps.table,
        });
        Ok(())
    }
}
