use borsh::BorshSerialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
};

use crate::{
    error::InglError,
    state::{consts::PROGRAM_STORAGE_SEED, LogLevel, Storage},
    utils::{get_clock_data, AccountInfoHelpers, OptionExt, ResultExt},
};

pub fn validate_secondary_items_transfers(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    _log_level: LogLevel,
    item_index: u32,
    clock_is_from_account: bool,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let payer_account_info = next_account_info(account_info_iter)?;
    let storage_account_info = next_account_info(account_info_iter)?;

    let clock_data = get_clock_data(account_info_iter, clock_is_from_account)?;

    payer_account_info.assert_signer()?;

    storage_account_info.assert_seed(program_id, &[PROGRAM_STORAGE_SEED])?;

    let mut storage_data = Storage::parse(storage_account_info, program_id)?;

    payer_account_info
        .assert_key_match(
            &storage_data
                .purchase
                .clone()
                .error_log("Validation cannot be performed before a purchase has been made")?
                .buyer,
        )
        .error_log("Only the buyer can validate secondary items transfers")?;

    if let Some(_date_validated) = storage_data.secondary_items[item_index as usize].date_validated
    {
        Err(InglError::TooLate.utilize("Secondary item has already been validated"))?
    }

    storage_data.secondary_items[item_index as usize].date_validated =
        Some(clock_data.unix_timestamp as u32);

    storage_data.serialize(&mut &mut storage_account_info.data.borrow_mut()[..])?;
    Ok(())
}
