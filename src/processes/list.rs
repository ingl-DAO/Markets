use solana_program::{account_info::AccountInfo, pubkey::Pubkey, entrypoint::ProgramResult};

use crate::state::LogLevel;

pub fn list_validator(program_id: &Pubkey, accounts: &[AccountInfo], log_level: LogLevel) -> ProgramResult{
    Ok(())
}