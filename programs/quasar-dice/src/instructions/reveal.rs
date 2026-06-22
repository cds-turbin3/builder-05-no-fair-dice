//! The reveal marker. It carries the preimage as instruction data and the house
//! as a signer; `resolve_bet` reads both back out of the Instructions sysvar.
//! The handler does nothing: its entire purpose is to exist in the transaction
//! one slot before `resolve_bet`.

use quasar_lang::prelude::*;

#[derive(Accounts)]
pub struct Reveal {
    pub house: Signer,
}

impl Reveal {
    #[inline(always)]
    pub fn handle(&self) -> Result<(), ProgramError> {
        Ok(())
    }
}
