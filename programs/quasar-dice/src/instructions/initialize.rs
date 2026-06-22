//! House funds its vault PDA: a plain system transfer from the (signing) house
//! into the program-addressed vault.

use {crate::state::VaultPda, quasar_lang::prelude::*};

#[derive(Accounts)]
pub struct Initialize {
    #[account(mut)]
    pub house: Signer,

    #[account(mut, address = VaultPda::seeds(house.address()))]
    pub vault: UncheckedAccount,

    pub system_program: Program<SystemProgram>,
}

impl Initialize {
    #[inline(always)]
    pub fn handle(&self, amount: u64) -> Result<(), ProgramError> {
        self.system_program
            .transfer(&self.house, &self.vault, amount)
            .invoke()
    }
}
