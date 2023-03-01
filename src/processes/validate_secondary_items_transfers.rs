use borsh::BorshSerialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program::invoke,
    pubkey::Pubkey,
    system_instruction,
};

use crate::{
    error::InglError,
    state::{
        consts::{ESCROW_ACCOUNT_SEED, PROGRAM_STORAGE_SEED},
        LogLevel, Storage,
    },
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
    let buyer_account_info = next_account_info(account_info_iter)?;
    let storage_account_info = next_account_info(account_info_iter)?;
    let escrow_account_info = next_account_info(account_info_iter)?;
    let authorized_withdrawer_info = next_account_info(account_info_iter)?;

    let clock_data = get_clock_data(account_info_iter, clock_is_from_account)?;

    buyer_account_info.assert_signer()?;

    storage_account_info
        .assert_seed(program_id, &[PROGRAM_STORAGE_SEED])
        .error_log("Error @ storage account pda assertion")?;
    escrow_account_info
        .assert_seed(program_id, &[ESCROW_ACCOUNT_SEED])
        .error_log("Error @ escrow account pda assertion")?;

    let mut storage_data = Storage::parse(storage_account_info, program_id)?;
    authorized_withdrawer_info
        .assert_key_match(&storage_data.authorized_withdrawer)
        .error_log("Error @ authorized withdrawer account assertion")?;

    if let Some(purchase_data) = &storage_data.purchase {
        if purchase_data.date_finalized.is_some() {
            Err(InglError::TooLate.utilize("Purchase has already been finalized"))?
        }
    }

    buyer_account_info
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

    let invalidated_secondary_items = storage_data
        .secondary_items
        .iter()
        .filter(|item| item.date_validated.is_none())
        .count();
    if invalidated_secondary_items == 0 {
        let secondary_items_cost = storage_data
            .secondary_items
            .iter()
            .map(|item| item.cost)
            .sum::<u64>();

        invoke(
            &system_instruction::transfer(
                &escrow_account_info.key,
                &buyer_account_info.key,
                secondary_items_cost,
            ),
            &[escrow_account_info.clone(), buyer_account_info.clone()],
        )?;
        invoke(
            &system_instruction::transfer(
                &escrow_account_info.key,
                &authorized_withdrawer_info.key,
                escrow_account_info.lamports(),
            ),
            &[
                escrow_account_info.clone(),
                authorized_withdrawer_info.clone(),
            ],
        )?;
        storage_data
            .purchase
            .error_log("purchase must have taken place")?
            .date_finalized = Some(clock_data.unix_timestamp as u32);
    }

    storage_data
        .serialize(&mut &mut storage_account_info.data.borrow_mut()[..])
        .error_log("Error @ storage_data.serialize")?;
    Ok(())
}
