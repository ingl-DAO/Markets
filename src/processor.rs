use borsh::BorshDeserialize;
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

use crate::{
    instruction::InstructionEnum,
    processes::{
        buy::buy_validator, delist::delist_validator, list::list_validator, mediate::mediate,
        request_mediation::request_mediation,
        validate_secondary_items_transfers::validate_secondary_items_transfers,
        withdraw_rewards::withdraw_rewards,
    },
};

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    let instruction = InstructionEnum::decode(data);

    match instruction {
        InstructionEnum::List {
            log_level,
            authorized_withdrawer_cost,
            mediatable_date,
            secondary_items,
            description,
        } => list_validator(
            program_id,
            accounts,
            authorized_withdrawer_cost,
            secondary_items,
            description,
            log_level,
            mediatable_date,
            false,
            false,
        )?,
        InstructionEnum::Delist { log_level } => delist_validator(program_id, accounts, log_level)?,
        InstructionEnum::Buy { log_level } => buy_validator(program_id, accounts, log_level)?,
        InstructionEnum::WithdrawRewards { log_level } => {
            withdraw_rewards(program_id, accounts, log_level)?
        }
        InstructionEnum::RequestMediation { log_level } => {
            request_mediation(program_id, accounts, log_level, false)?
        }
        InstructionEnum::Mediate { log_level } => mediate(program_id, accounts, log_level)?,
        InstructionEnum::ValidateSecondaryItemsTransfers { log_level } => {
            validate_secondary_items_transfers(program_id, accounts, log_level)?
        }
    }

    Ok(())
}
