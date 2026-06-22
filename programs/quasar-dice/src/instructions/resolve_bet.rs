//! The house settles a bet. This instruction must directly follow a `reveal` in
//! the same transaction: it introspects the Instructions sysvar to recover the
//! preimage and confirm the house signed the reveal, opens the commitment,
//! derives the roll, and pays out a win. The Bet account is closed to the player.

use {
    crate::{
        constants::HOUSE_EDGE_BASIS_POINTS,
        error::DiceError,
        introspection::{prev_reveal_preimage, INSTRUCTIONS_SYSVAR_ID},
        state::{Bet, VaultPda},
        REVEAL_DISCRIMINATOR,
    },
    quasar_lang::{cpi::Seed, prelude::*},
};

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct ResolveBet {
    #[account(mut)]
    pub house: Signer,

    /// The winner of a settled bet; receives the payout and the closed Bet's rent.
    #[account(mut)]
    pub player: UncheckedAccount,

    #[account(mut, address = VaultPda::seeds(house.address()))]
    pub vault: UncheckedAccount,

    #[account(
        mut,
        has_one(player),
        close(dest = player),
        address = Bet::seeds(vault.address(), player.address(), seed)
    )]
    pub bet: Account<Bet>,

    /// Verified by address in the handler before its data is parsed.
    pub instruction_sysvar: UncheckedAccount,

    pub system_program: Program<SystemProgram>,
}

impl ResolveBet {
    pub fn resolve(&mut self, bumps: &ResolveBetBumps) -> Result<(), ProgramError> {
        // 1. Recover the preimage from the preceding `reveal` instruction.
        let preimage = {
            let sysvar = self.instruction_sysvar.to_account_view();
            require_keys_eq!(
                *sysvar.address(),
                INSTRUCTIONS_SYSVAR_ID,
                DiceError::BadSignature
            );
            let data = sysvar.try_borrow()?;
            prev_reveal_preimage(
                &data,
                &crate::ID,
                self.house.address(),
                REVEAL_DISCRIMINATOR,
            )?
        };

        // 2. The preimage must open the stored commitment.
        require!(
            crate::sha256(&preimage) == self.bet.commitment,
            DiceError::CommitRevealMismatch
        );

        // 3. Derive the roll (1..=100) from BOTH parties: the house's hidden
        //    preimage and the player's open entropy. Neither fixes it alone, so
        //    the roll is unknowable until the house reveals.
        let mut mixed = [0u8; 64];
        mixed[..32].copy_from_slice(&preimage);
        mixed[32..].copy_from_slice(&self.bet.entropy);
        let roll_hash = crate::sha256(&mixed);

        let mut half = [0u8; 16];
        half.copy_from_slice(&roll_hash[0..16]);
        let lower = u128::from_le_bytes(half);
        half.copy_from_slice(&roll_hash[16..32]);
        let upper = u128::from_le_bytes(half);
        let roll = (lower.wrapping_add(upper).wrapping_rem(100) as u8) + 1;

        // 4. A guess strictly greater than the roll wins; the payout scales
        //    inversely with how many numbers the guess covers, less the edge.
        let guess = self.bet.guess_roll;
        if guess > roll {
            let amount = self.bet.amount.get();
            let winning_numbers = guess as u128 - 1;
            let payout = (amount as u128)
                .checked_mul(10_000 - HOUSE_EDGE_BASIS_POINTS as u128)
                .ok_or(DiceError::Overflow)?
                .checked_div(winning_numbers)
                .ok_or(DiceError::Overflow)?
                .checked_div(100)
                .ok_or(DiceError::Overflow)?;
            let payout = u64::try_from(payout).map_err(|_| DiceError::Overflow)?;

            // The vault PDA signs the outbound transfer.
            let house = *self.house.address();
            let bump = [bumps.vault];
            let seeds = [
                Seed::from(b"vault" as &[u8]),
                Seed::from(house.as_ref()),
                Seed::from(bump.as_ref()),
            ];
            self.system_program
                .transfer(&self.vault, &self.player, payout)
                .invoke_signed(&seeds)?;
        }

        // A losing bet pays nothing; the stake stays in the vault. Either way the
        // `close(dest = player)` constraint returns the Bet's rent to the player.
        Ok(())
    }
}
