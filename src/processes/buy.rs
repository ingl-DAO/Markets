use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

use crate::state::LogLevel;

pub fn buy_validator(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    log_level: LogLevel,
) -> ProgramResult {
    Ok(())
}
