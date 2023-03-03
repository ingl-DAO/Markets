use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    borsh::try_from_slice_unchecked,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
};

use crate::state::{consts, LogLevel, MediationShares, StoredSecondaryItem};

#[derive(BorshSerialize, BorshDeserialize)]
pub struct SecondaryItem {
    cost: u64,
    name: String,
    description: String,
}
impl SecondaryItem {
    pub fn to_stored(&self) -> StoredSecondaryItem {
        StoredSecondaryItem {
            cost: self.cost,
            name: self.name.clone(),
            description: self.description.clone(),
            date_validated: None,
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
pub enum InstructionEnum {
    List {
        log_level: LogLevel,
        authorized_withdrawer_cost: u64,
        mediatable_date: u32,
        secondary_items: Vec<SecondaryItem>,
        description: String,
        validator_name: String,
        validator_logo_url: String,
    },
    Delist {
        log_level: LogLevel,
    },
    Buy {
        log_level: LogLevel,
    },
    WithdrawRewards {
        log_level: LogLevel,
    },
    RequestMediation {
        log_level: LogLevel,
    },
    Mediate {
        log_level: LogLevel,
        mediation_shares: MediationShares,
    },
    ValidateSecondaryItemsTransfers {
        log_level: LogLevel,
        item_index: u32,
    },
}
impl InstructionEnum {
    pub fn decode(data: &[u8]) -> Self {
        try_from_slice_unchecked(data).expect("Failed during the Desrialization of InstructionEnum")
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
pub enum RegistryInstructionEnum {
    InitConfig,
    AddValidatorProgram { name: String },
    RemovePrograms { program_count: u8 },
    AddMarketplaceProgram,
    Reset,
    Blank,
}

pub fn register_program_instruction(payer: Pubkey, program_id: Pubkey) -> Instruction {
    let instr = RegistryInstructionEnum::AddMarketplaceProgram;
    let data = instr.try_to_vec().unwrap();
    // let config_key =
    //     Pubkey::find_program_address(&[b"config"], &constants::program_registry::id()).0;
    let (storage_key, _storage_bump) =
        Pubkey::find_program_address(&[b"storage"], &consts::program_registry::id());

    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(program_id, false),
        AccountMeta::new(consts::TEAM_ADDRESS, false),
        AccountMeta::new(storage_key, false),
        // AccountMeta::new(config_key, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    Instruction {
        program_id: consts::program_registry::id(),
        accounts: accounts,
        data,
    }
}
