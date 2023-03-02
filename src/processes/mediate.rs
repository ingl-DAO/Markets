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
    log,
    state::{
        consts::{ESCROW_ACCOUNT_SEED, MEDIATORS, PROGRAM_STORAGE_SEED, TEAM_ADDRESS},
        LogLevel, MediationShares, Storage,
    },
    utils::{get_clock_data, AccountInfoHelpers, OptionExt, ResultExt},
};

pub fn mediate(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    mediation_shares: MediationShares,
    log_level: LogLevel,
    clock_is_from_account: bool,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let payer_account_info = next_account_info(account_info_iter)?;
    let authorized_withdrawer_info = next_account_info(account_info_iter)?;
    let storage_account_info = next_account_info(account_info_iter)?;
    let buyer_account_info = next_account_info(account_info_iter)?;
    let escrow_account_info = next_account_info(account_info_iter)?;
    let team_account_info = next_account_info(account_info_iter)?;

    let clock_data = get_clock_data(account_info_iter, clock_is_from_account)?;

    payer_account_info
        .assert_signer()
        .error_log("Error @ payer_account_info.assert_signer")?;

    if !MEDIATORS.contains(payer_account_info.key) {
        Err(InglError::NotAuthorized.utilize("only approved mediators can mediate"))?
    }

    storage_account_info
        .assert_seed(program_id, &[PROGRAM_STORAGE_SEED])
        .error_log("Error @ storage_account_info.assert_seed")?;
    escrow_account_info
        .assert_seed(program_id, &[ESCROW_ACCOUNT_SEED])
        .error_log("Error @ escrow_account_info.assert_seed")?;

    team_account_info
        .assert_key_match(&TEAM_ADDRESS)
        .error_log("Error @ team_account_info.assert_key_match")?;

    let mut storage_data = Storage::parse(storage_account_info, program_id)?;

    if let Some(purchase_data) = &storage_data.purchase {
        if purchase_data.date_finalized.is_some() {
            Err(InglError::TooLate.utilize("Purchase has already been finalized"))?
        }
    }

    authorized_withdrawer_info
        .assert_key_match(&storage_data.authorized_withdrawer)
        .error_log(
            "Error @ authorized_withdrawer_info.assert_key_match(&storage_data.authorized_withdrawer)",
        )?;

    match storage_data.request_mediation_date {
        Some(_request_mediation_date) => {
            if storage_data.mediation_date.is_some() {
                Err(InglError::TooLate.utilize("Mediation has already taken place"))?
            }
        }
        None => Err(InglError::TooEarly.utilize("Mediation has not been requested yet"))?,
    }

    buyer_account_info
        .assert_key_match(
            &storage_data
                .purchase
                .error_log("mediation can only take place if purchase took place")?
                .buyer,
        )
        .error_log("Error @ buyer_account_info.assert_key_match(&storage_data.buyer)")?;

    storage_data.mediation_date = Some(clock_data.unix_timestamp as u32);
    storage_data
        .purchase
        .error_log("mediation can only take place if purchase took place")?
        .date_finalized = Some(clock_data.unix_timestamp as u32);

    mediation_shares.verify_sum()?;

    let to_buyer = escrow_account_info
        .lamports()
        .checked_mul(mediation_shares.buyer)
        .error_log("buyer share * escrow lamports overflows")?
        .checked_div(100)
        .error_log("buyer share * escrow lamports overflows / 100")?;
    let to_seller = escrow_account_info
        .lamports()
        .checked_mul(mediation_shares.seller)
        .error_log("seller share * escrow lamports overflows")?
        .checked_div(100)
        .error_log("seller share * escrow lamports overflows / 100")?;
    let to_team = escrow_account_info
        .lamports()
        .checked_sub(
            to_buyer
                .checked_add(to_seller)
                .error_log("to_buyer + to_seller overflows")?,
        )
        .error_log("escrow lamports - (to_buyer + to_seller) overflows")?;

    log!(
        log_level,
        3,
        "to_buyer: {}, to_seller: {}, to_team: {}",
        to_buyer,
        to_seller,
        to_team
    );

    let do_transfers = || -> ProgramResult {
        invoke(
            &system_instruction::transfer(
                escrow_account_info.key,
                authorized_withdrawer_info.key,
                to_seller,
            ),
            &[
                escrow_account_info.clone(),
                authorized_withdrawer_info.clone(),
            ],
        )
        .error_log("Error @ transfer to seller")?;

        if to_buyer > 0 {
            invoke(
                &system_instruction::transfer(
                    escrow_account_info.key,
                    buyer_account_info.key,
                    to_buyer,
                ),
                &[escrow_account_info.clone(), buyer_account_info.clone()],
            )
            .error_log("Error @ transfer to buyer")?;
        }

        if to_team > 0 {
            invoke(
                &system_instruction::transfer(
                    escrow_account_info.key,
                    team_account_info.key,
                    to_team,
                ),
                &[escrow_account_info.clone(), team_account_info.clone()],
            )
            .error_log("Error @ transfer to team")?;
        }
        Ok(())
    };

    do_transfers().error_log("Error @ do_transfer")?;

    storage_data
        .serialize(&mut &mut storage_account_info.data.borrow_mut()[..])
        .error_log("Error @ storage_data.serialize")?;

    Ok(())
}
