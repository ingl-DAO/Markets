use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program::invoke_signed,
    pubkey::Pubkey,
    vote,
};

use crate::{
    state::{
        consts::{PDA_AUTHORIZED_WITHDRAWER_SEED, PROGRAM_STORAGE_SEED},
        LogLevel, Storage, VoteState,
    },
    utils::{AccountInfoHelpers, OptionExt, ResultExt},
};

pub fn withdraw_rewards(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    _log_level: LogLevel,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let authorized_withdrawer_info = next_account_info(account_info_iter)?;
    let vote_account_info = next_account_info(account_info_iter)?;
    let pda_authorized_withdrawer_info = next_account_info(account_info_iter)?;
    let storage_account_info = next_account_info(account_info_iter)?;

    let (_pda_authority_key, pda_authority_bump) = pda_authorized_withdrawer_info
        .assert_seed(program_id, &[PDA_AUTHORIZED_WITHDRAWER_SEED])
        .error_log("Error @ pda_authorized_withdrawer_info.assert_seed")?;
    storage_account_info
        .assert_seed(program_id, &[PROGRAM_STORAGE_SEED])
        .error_log("Error @ storage_account_info.assert_seed")?;

    let storage_data = Storage::parse(storage_account_info, program_id)?;

    vote_account_info
        .assert_owner(&vote::program::id())
        .error_log("Error @ vote_account_info.assert_owner")?;
    vote_account_info
        .assert_key_match(&storage_data.vote_account)
        .error_log("Error @ vote_account_info.assert_key_match(&storage_data.vote_account)")?;

    authorized_withdrawer_info
        .assert_key_match(&storage_data.authorized_withdrawer)
        .error_log(
            "Error @ authorized_withdrawer_info.assert_key_match(&storage_data.authorized_withdrawer)",
        )?;

    let lamports = vote_account_info
        .lamports()
        .checked_sub(VoteState::min_lamports())
        .error_log("Error @ vote_account_info.lamports().checked_sub(VoteState::min_lamports())")?;

    invoke_signed(
        &vote::instruction::withdraw(
            vote_account_info.key,
            pda_authorized_withdrawer_info.key,
            lamports,
            authorized_withdrawer_info.key,
        ),
        &[
            vote_account_info.clone(),
            authorized_withdrawer_info.clone(),
            pda_authorized_withdrawer_info.clone(),
        ],
        &[&[PDA_AUTHORIZED_WITHDRAWER_SEED, &[pda_authority_bump]]],
    )?;

    Ok(())
}
