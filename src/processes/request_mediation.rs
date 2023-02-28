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

pub fn request_mediation(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    _log_level: LogLevel,
    clock_is_from_account: bool,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let payer_account_info = next_account_info(account_info_iter)?;
    let storage_account_info = next_account_info(account_info_iter)?;

    let clock_data = get_clock_data(account_info_iter, clock_is_from_account)?;

    payer_account_info.assert_signer()?;

    storage_account_info
        .assert_seed(program_id, &[PROGRAM_STORAGE_SEED])
        .error_log("Error @ storage pda validation")?;

    let mut storage_data = Storage::parse(storage_account_info, program_id)?;
    let purchase_data = storage_data
        .purchase
        .clone()
        .error_log("One must wait for a purchase to take place before requesting mediation")?;

    if *payer_account_info.key != storage_data.authorized_withdrawer
        && *payer_account_info.key != purchase_data.buyer
    {
        Err(InglError::NotAuthorized.utilize("Only the buyer or the seller can request mediation"))?
    }

    if (clock_data.unix_timestamp as u32) < storage_data.mediatable_date {
        Err(InglError::TooEarly.utilize("Mediation cannot be requested yet"))?
    }

    if let Some(_request_mediation_date) = storage_data.request_mediation_date {
        Err(InglError::TooLate.utilize("Mediation has already been requested"))?
    }

    storage_data.request_mediation_date = Some(clock_data.unix_timestamp as u32);

    storage_data.serialize(&mut &mut storage_account_info.data.borrow_mut()[..])?;

    Ok(())
}
