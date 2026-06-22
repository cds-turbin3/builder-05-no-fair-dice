//! Instruction introspection over the Instructions sysvar.
//!
//! Quasar ships no introspection helper, and the stock
//! `solana-instructions-sysvar` loaders take `solana_program`'s `AccountInfo`,
//! not Quasar's `AccountView`. So we parse the sysvar account data directly. The
//! byte layout is the standard one produced by `construct_instructions_data`
//! (which Quasar's runtime uses), documented here so the parse is byte-exact:
//!
//! ```text
//! Header
//!   [0..2]                    num_instructions (u16 LE)
//!   [2..2 + 2*N]              instruction offset table ([u16 LE; N])
//!   [len-2..len]              current_index (u16 LE)
//! Each instruction (at its offset)
//!   [0..2]                    num_accounts (u16 LE)
//!   [2..2 + 33*A]             accounts: A * { meta:u8 (bit0=signer,bit1=writable), pubkey:[u8;32] }
//!   [.. + 32]                 program_id ([u8;32])
//!   [.. + 2]                  data_len (u16 LE)
//!   [.. + data_len]           data
//! ```
//!
//! Prototyped with LLM assistance; the layout is exercised by the dogfood suite.
//! TODO: validate for correctness beyond testing (e.g. a proptest over this layout,
//! or a cross-check against `construct_instructions_data`).

use quasar_lang::prelude::*;

/// The Instructions sysvar address (`Sysvar1nstructions1111111111111111111111111`).
pub const INSTRUCTIONS_SYSVAR_ID: Address =
    address!("Sysvar1nstructions1111111111111111111111111");

fn read_u16(data: &[u8], at: usize) -> Result<u16, ProgramError> {
    let b = data
        .get(at..at + 2)
        .ok_or(ProgramError::InvalidInstructionData)?;
    Ok(u16::from_le_bytes([b[0], b[1]]))
}

/// Take the instruction immediately preceding the current one, verify it is a
/// `reveal` of `program_id` signed by `house`, and return the 32-byte preimage
/// it carries.
///
/// Pins the introspected program id and the `reveal` discriminator alongside the
/// house signature, so a same-transaction instruction that merely happens to
/// carry the house as a signer cannot be mistaken for a reveal.
pub fn prev_reveal_preimage(
    sysvar_data: &[u8],
    program_id: &Address,
    house: &Address,
    reveal_discriminator: u8,
) -> Result<[u8; 32], ProgramError> {
    let len = sysvar_data.len();
    if len < 2 {
        return Err(ProgramError::InvalidInstructionData);
    }
    let current = u16::from_le_bytes([sysvar_data[len - 2], sysvar_data[len - 1]]);
    if current == 0 {
        // No preceding instruction to introspect.
        return Err(ProgramError::InvalidInstructionData);
    }
    let target = (current - 1) as usize;

    let num_instructions = read_u16(sysvar_data, 0)? as usize;
    if target >= num_instructions {
        return Err(ProgramError::InvalidInstructionData);
    }

    // Walk to the target instruction via its offset-table entry.
    let offset = read_u16(sysvar_data, 2 + target * 2)? as usize;
    let mut cur = offset;
    let num_accounts = read_u16(sysvar_data, cur)? as usize;
    cur += 2;

    let mut house_is_signer = false;
    for _ in 0..num_accounts {
        let meta = *sysvar_data
            .get(cur)
            .ok_or(ProgramError::InvalidInstructionData)?;
        let pubkey = sysvar_data
            .get(cur + 1..cur + 33)
            .ok_or(ProgramError::InvalidInstructionData)?;
        if meta & 0b1 != 0 && pubkey == house.as_ref() {
            house_is_signer = true;
        }
        cur += 33;
    }

    let prog = sysvar_data
        .get(cur..cur + 32)
        .ok_or(ProgramError::InvalidInstructionData)?;
    cur += 32;
    let data_len = read_u16(sysvar_data, cur)? as usize;
    cur += 2;
    let ix_data = sysvar_data
        .get(cur..cur + data_len)
        .ok_or(ProgramError::InvalidInstructionData)?;

    if prog != program_id.as_ref() {
        return Err(crate::error::DiceError::NotReveal.into());
    }
    if !house_is_signer {
        return Err(crate::error::DiceError::BadSignature.into());
    }
    // [discriminator: u8][preimage: [u8; 32]]
    if ix_data.first() != Some(&reveal_discriminator) {
        return Err(crate::error::DiceError::NotReveal.into());
    }
    let preimage = ix_data
        .get(1..33)
        .ok_or(ProgramError::InvalidInstructionData)?;
    let mut out = [0u8; 32];
    out.copy_from_slice(preimage);
    Ok(out)
}
