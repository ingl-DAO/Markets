use borsh::BorshSerialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    bpf_loader_upgradeable,
    entrypoint::ProgramResult,
    program::invoke,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction, sysvar,
    vote::{self, instruction::authorize, state::VoteAuthorize},
};

use crate::{
    error::InglError,
    instruction::{register_program_instruction, SecondaryItem},
    log,
    state::{
        consts::{
            PDA_AUTHORIZED_WITHDRAWER_SEED, PDA_UPGRADE_AUTHORITY_SEED, PROGRAM_STORAGE_SEED,
            STORAGE_VALIDATION_PHRASE,
        },
        LogLevel, Storage, VoteState,
    },
    utils::{get_clock_data_from_account, get_rent_data, AccountInfoHelpers, ResultExt},
};

pub fn list_validator(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    authorized_withdrawer_cost: u64,
    secondary_items: Vec<SecondaryItem>,
    description: String,
    log_level: LogLevel,
    mediatable_date: u32,
    validator_name: String,
    validator_logo_url: String,
    rent_is_from_account: bool,
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

    let team_account_info = next_account_info(account_info_iter)?;
    let registry_storage_account_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;

    let rent_data = get_rent_data(account_info_iter, rent_is_from_account)?;
    let clock_data = get_clock_data_from_account(sysvar_clock_account_info)?;

    if mediatable_date > (clock_data.unix_timestamp + 30 * 86400) as u32 {
        Err(InglError::TooLate.utilize("Mediatable date can't be more than 30 days in the future"))?
    }

    if validator_name.is_empty() {
        Err(InglError::InvalidData.utilize("Validator name can't be empty"))?
    }

    log!(
        log_level,
        2,
        "list_validator: verify_and_change_program_authority"
    );
    verify_and_change_program_authority(
        program_id,
        current_upgrade_authority_info,
        pda_upgrade_authority_info,
        this_program_account_info,
        this_program_data_account_info,
    )?;

    log!(
        log_level,
        2,
        "list_validator: verify_and_change_authorized_withdrawer"
    );
    verify_and_change_authorized_withdrawer(
        program_id,
        vote_account_info,
        authorized_withdrawer_info,
        pda_authorized_withdrawer_info,
        sysvar_clock_account_info,
    )?;

    log!(
        log_level,
        2,
        "list_validator: create_storage_and_store_data"
    );
    create_storage_and_store_data(
        program_id,
        storage_account_info,
        authorized_withdrawer_info,
        vote_account_info,
        authorized_withdrawer_cost,
        secondary_items,
        description,
        validator_name,
        validator_logo_url,
        mediatable_date,
        rent_data,
    )?;

    let registry_program_accounts = vec![
        authorized_withdrawer_info.clone(),
        this_program_account_info.clone(),
        team_account_info.clone(),
        registry_storage_account_info.clone(),
        // registry_program_config_account.clone(),
        system_program_info.clone(),
    ];

    log!(log_level, 2, "Initing Program Registration ... ");
    invoke(
        &register_program_instruction(*authorized_withdrawer_info.key, *program_id),
        &registry_program_accounts,
    )?;

    Ok(())
}

pub fn create_storage_and_store_data<'a>(
    program_id: &Pubkey,
    storage_account: &AccountInfo<'a>,
    payer_account: &AccountInfo<'a>,
    vote_account: &AccountInfo<'a>,
    cost: u64,
    secondary_items: Vec<SecondaryItem>,
    description: String,
    validator_name: String,
    validator_logo_url: String,
    mediatable_date: u32,
    rent_data: Rent,
) -> ProgramResult {
    storage_account
        .assert_seed(program_id, &[PROGRAM_STORAGE_SEED])
        .error_log("Error @ storage_account_info.assert_seed")?;

    let storage_data = Storage {
        validation_phrase: STORAGE_VALIDATION_PHRASE,
        authorized_withdrawer: *payer_account.key,
        vote_account: *vote_account.key,
        authorized_withdrawer_cost: cost,
        request_mediation_date: None,
        mediation_date: None,
        mediation_shares: None,
        secondary_items: secondary_items
            .iter()
            .map(|item| item.to_stored())
            .collect(),
        description: description,
        validator_name: validator_name,
        validator_logo_url: validator_logo_url,
        purchase: None,
        mediatable_date,
    };

    let space = storage_data.get_space();
    let lamports = rent_data.minimum_balance(space);
    invoke(
        &system_instruction::create_account(
            &payer_account.key,
            &storage_account.key,
            lamports,
            space as u64,
            program_id,
        ),
        &[payer_account.clone(), storage_account.clone()],
    )
    .error_log("Error @ system_instruction::create_account")?;

    storage_data
        .serialize(&mut &mut storage_account.data.borrow_mut()[..])
        .error_log("Error @ storage_data.serialize")?;

    Ok(())
}

pub fn verify_and_change_authorized_withdrawer<'a>(
    program_id: &Pubkey,
    vote_account: &AccountInfo<'a>,
    current_authorized_withdrawer: &AccountInfo<'a>,
    pda_authorized_withdrawer: &AccountInfo<'a>,
    sysvar_clock_account: &AccountInfo<'a>,
) -> ProgramResult {
    current_authorized_withdrawer
        .assert_signer()
        .error_log("Error @ current_authorized_withdrawer.assert_signer")?;
    vote_account
        .assert_owner(&vote::program::ID)
        .error_log("vote_account must be owned by vote_program")?;
    let vote_account_data = VoteState::deserialize(&vote_account.data.borrow());
    current_authorized_withdrawer
        .assert_key_match(&vote_account_data.authorized_withdrawer)
        .error_log("Error @ current_authorized_withdrawer.assert_key_match")?;
    sysvar_clock_account
        .assert_key_match(&sysvar::clock::id())
        .error_log("Error @ sysvar_clock_account.assert_key_match")?;

    pda_authorized_withdrawer
        .assert_seed(program_id, &[PDA_AUTHORIZED_WITHDRAWER_SEED])
        .error_log("Error @ pda_authorized_withdrawer_info.assert_seed")?;

    invoke(
        &authorize(
            &vote_account.key,
            &current_authorized_withdrawer.key,
            &pda_authorized_withdrawer.key,
            VoteAuthorize::Withdrawer,
        ),
        &[
            vote_account.clone(),
            sysvar_clock_account.clone(),
            current_authorized_withdrawer.clone(),
        ],
    )
    .error_log("Error switching authorized withdrawer")?;

    Ok(())
}

pub fn verify_and_change_program_authority<'a>(
    program_id: &Pubkey,
    current_authority: &AccountInfo<'a>,
    pda_authority: &AccountInfo<'a>,
    this_program: &AccountInfo,
    this_program_data: &AccountInfo<'a>,
) -> ProgramResult {
    this_program
        .assert_owner(&bpf_loader_upgradeable::id())
        .error_log("Error @ program owner assertion")?;
    current_authority
        .assert_signer()
        .error_log("Error @ current_authority.assert_signer")?;
    this_program_data
        .assert_seed(&bpf_loader_upgradeable::id(), &[this_program.key.as_ref()])
        .error_log("Error @ program data key assertion")?;

    current_authority
        .assert_key_match(&Box::new(
            Pubkey::try_from(
                &this_program_data.data.borrow()[13..45], // Upgrade authority of the program
            )
            .expect("can't fetch upgrade authority"),
        ))
        .error_log("Error @ authority key assertion")?;

    this_program
        .assert_key_match(program_id)
        .error_log("Error @ program key assertion")?;

    pda_authority
        .assert_seed(program_id, &[PDA_UPGRADE_AUTHORITY_SEED])
        .error_log("Error @ pda_upgrade_authority.assert_seed")?;

    invoke(
        &bpf_loader_upgradeable::set_upgrade_authority(
            this_program.key,
            current_authority.key,
            Some(pda_authority.key),
        ),
        &[
            this_program_data.clone(),
            current_authority.clone(),
            pda_authority.clone(),
        ],
    )
    .error_log("Error setting upgrade authority")?;

    Ok(())
}
