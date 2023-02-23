use solana_program::{pubkey::Pubkey, entrypoint::ProgramResult, account_info::AccountInfo};

use crate::state::LogLevel;

pub fn withdraw_rewards(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    log_level: LogLevel,
) -> ProgramResult {
    Ok(())
}
