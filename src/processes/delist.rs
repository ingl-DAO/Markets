use solana_program::{
    account_info::{next_account_info, AccountInfo},
    bpf_loader_upgradeable,
    entrypoint::ProgramResult,
    program::invoke_signed,
    pubkey::Pubkey,
    sysvar, vote,
};

use crate::{
    error::InglError,
    instruction::vote_authorize,
    log,
    state::{
        consts::{
            PDA_AUTHORIZED_WITHDRAWER_SEED, PDA_UPGRADE_AUTHORITY_SEED, PROGRAM_STORAGE_SEED,
        },
        LogLevel, Storage, VoteAuthorize,
    },
    utils::{AccountInfoHelpers, OptionExt, ResultExt},
};

pub fn delist_validator(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    log_level: LogLevel,
) -> ProgramResult {
    log!(log_level, 4, "list_validator called");
    let account_info_iter = &mut accounts.iter();
    let authorized_withdrawer_info = next_account_info(account_info_iter)?;
    let vote_account_info = next_account_info(account_info_iter)?;
    let pda_authorized_withdrawer_info = next_account_info(account_info_iter)?;
    let storage_account_info = next_account_info(account_info_iter)?;
    let this_program_account_info = next_account_info(account_info_iter)?;
    let this_program_data_account_info = next_account_info(account_info_iter)?;
    let current_upgrade_authority_info = next_account_info(account_info_iter)?;
    let pda_upgrade_authority_info = next_account_info(account_info_iter)?;
    let sysvar_clock_account_info = next_account_info(account_info_iter)?;

    log!(
        log_level,
        2,
        "delist_validator: change_program_authority"
    );
    change_program_authority(
        program_id,
        current_upgrade_authority_info,
        pda_upgrade_authority_info,
        this_program_account_info,
        this_program_data_account_info,
    )?;

    log!(
        log_level,
        2,
        "delist_validator: change_authorized_withdrawer"
    );
    change_authorized_withdrawer(
        program_id,
        vote_account_info,
        authorized_withdrawer_info,
        pda_authorized_withdrawer_info,
        sysvar_clock_account_info,
    )?;

    log!(log_level, 2, "delist_validator: closing storage");
    verify_and_close_storage(
        program_id,
        storage_account_info,
        authorized_withdrawer_info,
        vote_account_info,
    )?;

    Ok(())
}

pub fn verify_and_close_storage<'a>(
    program_id: &Pubkey,
    storage_account: &AccountInfo<'a>,
    payer_account: &AccountInfo<'a>,
    vote_account: &AccountInfo<'a>,
) -> ProgramResult {
    storage_account
        .assert_seed(program_id, &[PROGRAM_STORAGE_SEED])
        .error_log("Error @ storage_account_info.assert_seed")?;
    payer_account
        .assert_signer()
        .error_log("Error @ payer_account_info.assert_signer")?;

    let storage_data =
        Storage::parse(storage_account, program_id).error_log("Error @ Storage::parse")?;

    payer_account
        .assert_key_match(&storage_data.authorized_withdrawer)
        .error_log("Error @ payer_account_info.assert_key_match")?;

    vote_account
        .assert_key_match(&storage_data.vote_account)
        .error_log("Error @ vote_account_info.assert_key_match")?;

    if let Some(_purchase) = storage_data.purchase {
        for item in &storage_data.secondary_items {
            match item.date_validated {
                None => Err(InglError::TooEarly
                    .utilize("One must wait for all secondary item transfers to be finalized"))?,
                Some(_) => (),
            }
        }
    }

    let storage_account_lamports = storage_account.lamports();
    **storage_account.lamports.borrow_mut() = 0;
    **payer_account.lamports.borrow_mut() = payer_account
        .lamports
        .borrow()
        .checked_add(storage_account_lamports)
        .error_log("Error adding storage lamports to payer")?;
    let mut storage_data = storage_account.data.borrow_mut();
    storage_data.fill(0);
    Ok(())
}

pub fn change_authorized_withdrawer<'a>(
    program_id: &Pubkey,
    vote_account: &AccountInfo<'a>,
    authorized_withdrawer: &AccountInfo<'a>,
    pda_authorized_withdrawer: &AccountInfo<'a>,
    sysvar_clock_account: &AccountInfo<'a>,
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
        &vote_authorize(
            &vote_account.key,
            &pda_authorized_withdrawer.key,
            &authorized_withdrawer.key,
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

pub fn change_program_authority<'a>(
    program_id: &Pubkey,
    authorized_withdrawer: &AccountInfo<'a>,
    pda_authority: &AccountInfo<'a>,
    this_program: &AccountInfo,
    this_program_data: &AccountInfo<'a>,
) -> ProgramResult {
    this_program
        .assert_owner(&bpf_loader_upgradeable::id())
        .error_log("Error @ program owner assertion")?;
    this_program_data
        .assert_seed(&bpf_loader_upgradeable::id(), &[this_program.key.as_ref()])
        .error_log("Error @ program data key assertion")?;

    this_program
        .assert_key_match(program_id)
        .error_log("Error @ program key assertion")?;

    let (_pda_authority_key, pda_authority_bump) = pda_authority
        .assert_seed(program_id, &[PDA_UPGRADE_AUTHORITY_SEED])
        .error_log("Error @ pda_upgrade_authority.assert_seed")?;

    invoke_signed(
        &bpf_loader_upgradeable::set_upgrade_authority(
            this_program.key,
            pda_authority.key,
            Some(authorized_withdrawer.key),
        ),
        &[
            this_program_data.clone(),
            pda_authority.clone(),
            authorized_withdrawer.clone(),
        ],
        &[&[PDA_UPGRADE_AUTHORITY_SEED, &[pda_authority_bump]]],
    )
    .error_log("Error setting upgrade authority")?;

    Ok(())
}
