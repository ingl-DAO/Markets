use borsh::BorshSerialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    program::{invoke, invoke_signed},
    pubkey::Pubkey,
    system_instruction, sysvar,
    vote::{self, instruction::authorize, state::VoteAuthorize},
};

use crate::{
    error::InglError,
    state::{
        consts::{
            ESCROWED_BASIS_POINTS, ESCROW_ACCOUNT_SEED, PDA_AUTHORIZED_WITHDRAWER_SEED,
            PROGRAM_STORAGE_SEED, TEAM_ADDRESS, TEAM_FEES_BASIS_POINTS,
        },
        LogLevel, Purchase, Storage,
    },
    utils::{get_clock_data_from_account, AccountInfoHelpers, OptionExt, ResultExt},
};

pub fn buy_validator(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    log_level: LogLevel,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let payer_account_info = next_account_info(account_info_iter)?;
    let storage_account_info = next_account_info(account_info_iter)?;
    let registered_authorized_withdrawer_info = next_account_info(account_info_iter)?;
    let vote_account_info = next_account_info(account_info_iter)?;
    let sysvar_clock_account_info = next_account_info(account_info_iter)?;
    let pda_authorized_withdrawer_info = next_account_info(account_info_iter)?;
    let escrow_account_info = next_account_info(account_info_iter)?;
    let team_account_info = next_account_info(account_info_iter)?;

    let clock_data =
        get_clock_data_from_account(sysvar_clock_account_info).error_log("Error @ clock parse")?;

    verify_transfer_cost_and_edit_storage(
        program_id,
        payer_account_info,
        storage_account_info,
        registered_authorized_withdrawer_info,
        escrow_account_info,
        team_account_info,
        &clock_data,
        log_level,
    )
    .error_log("Error @ verify_transfer_cost_and_edit_storage")?;

    change_authorized_withdrawer(
        program_id,
        vote_account_info,
        payer_account_info,
        pda_authorized_withdrawer_info,
        sysvar_clock_account_info,
        log_level,
    )
    .error_log("Error @ change_authorized_withdrawer")?;

    Ok(())
}

pub fn verify_transfer_cost_and_edit_storage<'a>(
    program_id: &Pubkey,
    payer_account: &AccountInfo<'a>,
    storage_account: &AccountInfo<'a>,
    registered_authorized_withdrawer: &AccountInfo<'a>,
    escrow_account: &AccountInfo<'a>,
    team_account: &AccountInfo<'a>,
    clock_data: &Clock,
    _log_level: LogLevel,
) -> ProgramResult {
    storage_account
        .assert_seed(program_id, &[PROGRAM_STORAGE_SEED])
        .error_log("Error @ storage pda validation")?;
    let mut storage_data =
        Storage::parse(storage_account, program_id).error_log("Error @ storage parse")?;
    registered_authorized_withdrawer
        .assert_key_match(&storage_data.authorized_withdrawer)
        .error_log("Error @ registered_authorized_withdrawer match")?;
    escrow_account
        .assert_seed(program_id, &[ESCROW_ACCOUNT_SEED])
        .error_log("Error @ escrow pda validation")?;

    team_account
        .assert_key_match(&TEAM_ADDRESS)
        .error_log("Error @ team match")?;

    if let Some(_purchase) = storage_data.purchase {
        Err(InglError::TooLate.utilize("Error @ validator is already bought"))?
    }

    let secondary_item_cost = storage_data
        .secondary_items
        .iter()
        .map(|item| item.cost)
        .sum::<u64>();

    let to_owner: u64 = (storage_data.authorized_withdrawer_cost as u128)
        .checked_mul(
            (10000
                - (if storage_data.secondary_items.len() > 0 {
                    ESCROWED_BASIS_POINTS
                } else {
                    0
                })
                - TEAM_FEES_BASIS_POINTS)
                .into(),
        )
        .error_log("to_owner mul calculation error")?
        .checked_div(10000)
        .error_log("to_owner div calculation error")? as u64;
    let to_escrow: u64 = secondary_item_cost * 2
        + (storage_data.authorized_withdrawer_cost as u128)
            .checked_mul(if storage_data.secondary_items.len() > 0 {
                ESCROWED_BASIS_POINTS.into()
            } else {
                0
            })
            .error_log("to_escrow mul calculation error")?
            .checked_div(10000)
            .error_log("to_escrow div calculation error")? as u64;
    let to_team: u64 = (storage_data.authorized_withdrawer_cost as u128)
        .checked_mul(TEAM_FEES_BASIS_POINTS.into())
        .error_log("to_team mul calculation error")?
        .checked_div(10000)
        .error_log("to_team div calculation error")? as u64;

    let do_transfers = || -> ProgramResult {
        invoke(
            &system_instruction::transfer(
                payer_account.key,
                registered_authorized_withdrawer.key,
                to_owner,
            ),
            &[
                payer_account.clone(),
                registered_authorized_withdrawer.clone(),
            ],
        )
        .error_log("Error @ transfer to owner")?;

        if to_escrow > 0 {
            invoke(
                &system_instruction::transfer(payer_account.key, escrow_account.key, to_escrow),
                &[payer_account.clone(), escrow_account.clone()],
            )
            .error_log("Error @ transfer to escrow")?;
        }

        if to_team > 0 {
            invoke(
                &system_instruction::transfer(payer_account.key, team_account.key, to_team),
                &[payer_account.clone(), team_account.clone()],
            )
            .error_log("Error @ transfer to team")?;
        }
        Ok(())
    };

    do_transfers().error_log("Error @ do_transfer")?;

    storage_data.purchase = Some(Purchase {
        buyer: *payer_account.key,
        date: clock_data.unix_timestamp as u32,
        date_finalized: if storage_data.secondary_items.is_empty() {
            Some(clock_data.unix_timestamp as u32)
        } else {
            None
        },
    });

    storage_data
        .serialize(&mut &mut storage_account.data.borrow_mut()[..])
        .error_log("Error @ storage serialize")?;
    Ok(())
}

pub fn change_authorized_withdrawer<'a>(
    program_id: &Pubkey,
    vote_account: &AccountInfo<'a>,
    new_authorized_withdrawer: &AccountInfo<'a>,
    pda_authorized_withdrawer: &AccountInfo<'a>,
    sysvar_clock_account: &AccountInfo<'a>,
    _log_level: LogLevel,
) -> ProgramResult {
    vote_account
        .assert_owner(&vote::program::ID)
        .error_log("vote_account must be owned by vote_program")?;

    sysvar_clock_account
        .assert_key_match(&sysvar::clock::id())
        .error_log("Error @ sysvar_clock_account.assert_key_match")?;

    let (_pda_authorized_withdrawer_key, pda_aw_bump) = pda_authorized_withdrawer
        .assert_seed(program_id, &[PDA_AUTHORIZED_WITHDRAWER_SEED])
        .error_log("Error @ pda_authorized_withdrawer_info.assert_seed")?;

    invoke_signed(
        &authorize(
            &vote_account.key,
            &pda_authorized_withdrawer.key,
            &new_authorized_withdrawer.key,
            VoteAuthorize::Withdrawer,
        ),
        &[
            vote_account.clone(),
            sysvar_clock_account.clone(),
            pda_authorized_withdrawer.clone(),
        ],
        &[&[PDA_AUTHORIZED_WITHDRAWER_SEED, &[pda_aw_bump]]],
    )
    .error_log("Error switching authorized withdrawer")?;

    Ok(())
}
