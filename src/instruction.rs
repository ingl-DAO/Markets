use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::borsh::try_from_slice_unchecked;

use crate::state::{LogLevel, MediationShares, StoredSecondaryItem};

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
