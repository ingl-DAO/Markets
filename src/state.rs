#![allow(unused_parens)]
use crate::{
    colored_log,
    error::InglError,
    utils::{AccountInfoHelpers, ResultExt},
};
use borsh::{BorshDeserialize, BorshSerialize};
use ingl_macros::Validate;
use serde::{Deserialize, Serialize};
use solana_program::{
    account_info::AccountInfo, borsh::try_from_slice_unchecked, program_error::ProgramError,
    pubkey::Pubkey, rent::Rent, slot_history::Slot, stake_history::Epoch, sysvar::Sysvar,
};
use std::collections::{BTreeMap, VecDeque};

use crate::state::LogColors::*;

pub mod consts {
    use solana_program::pubkey;
    use solana_program::pubkey::Pubkey;

    pub const PDA_AUTHORIZED_WITHDRAWER_SEED: &[u8] = b"authorized_withdrawer";
    pub const PROGRAM_STORAGE_SEED: &[u8] = b"program_storage";
    pub const PDA_UPGRADE_AUTHORITY_SEED: &[u8] = b"upgrade_authority";
    pub const ESCROW_ACCOUNT_SEED: &[u8] = b"escrow_account";
    pub const REGISTRY_STORAGE_SEED: &[u8] = b"marketplace_storage";

    pub const ESCROWED_BASIS_POINTS: u16 = 2000;
    pub const TEAM_FEES_BASIS_POINTS: u16 = 10;

    pub const STORAGE_VALIDATION_PHRASE: u32 = 838_927_652;

    pub const TEAM_ADDRESS: Pubkey = pubkey!("Et2tm6NsfBZJbEYXtWTv9k51V4tWtQvufexSgXoDRGVA");
    pub const MEDIATORS: [Pubkey; 1] = [pubkey!("Et2tm6NsfBZJbEYXtWTv9k51V4tWtQvufexSgXoDRGVA")];

    pub mod program_registry {

        use solana_program::declare_id;
        declare_id!("38pfsot7kCZkrttx1THEDXEz4JJXmCCcaDoDieRtVuy5");
    }
}

const LOG_LEVEL: u8 = 5;

#[derive(BorshDeserialize, BorshSerialize, Debug, Validate)]
#[validation_phrase(crate::state::consts::STORAGE_VALIDATION_PHRASE)]
pub struct Storage {
    pub validation_phrase: u32,
    pub authorized_withdrawer: Pubkey,
    pub vote_account: Pubkey,
    pub authorized_withdrawer_cost: u64,
    pub mediation_interval: u32,
    pub purchase: Option<Purchase>,
    pub request_mediation_date: Option<u32>,
    pub mediation_date: Option<u32>,
    pub mediation_shares: Option<MediationShares>,
    pub secondary_items: Vec<StoredSecondaryItem>,
    pub description: String,
    pub validator_name: String,
    pub validator_logo_url: String,
}

impl Storage {
    pub fn get_space(&self) -> usize {
        4 + 32
            + 32
            + 8
            + 4
            + 1
            + Purchase::get_space()
            + 5
            + 1
            + MediationShares::get_space()
            + 4
            + self
                .secondary_items
                .iter()
                .map(|x| x.get_space())
                .sum::<usize>()
            + 4
            + self.description.len()
            + 4
            + self.validator_name.len()
            + 1
            + 4
            + self.validator_logo_url.len()
    }
}

#[derive(BorshSerialize, BorshDeserialize, Copy, Clone, Debug)]
pub struct Purchase {
    pub buyer: Pubkey,
    pub date: u32,
    pub date_finalized: Option<u32>,
}

impl Purchase {
    pub fn get_space() -> usize {
        32 + 4 + 5
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct StoredSecondaryItem {
    pub cost: u64,
    pub name: String,
    pub description: String,
    pub date_validated: Option<u32>,
}

impl StoredSecondaryItem {
    pub fn get_space(&self) -> usize {
        8 + self.name.len() + 4 + self.description.len() + 4 + 5
    }
}

pub enum LogColors {
    Red,
    Green,
    Blue,
    Blank,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct MediationShares {
    pub buyer: u8,
    pub seller: u8,
    pub team: u8,
}

impl MediationShares {
    pub fn verify_sum(&self) -> Result<(), ProgramError> {
        if self.buyer + self.seller + self.team != 100 {
            Err(InglError::InvalidData.utilize("mediation shares do not sum to 100"))?
        }
        Ok(())
    }
    pub fn get_space() -> usize {
        8 + 8 + 8
    }
}

#[derive(BorshDeserialize, Clone)]
pub struct VoteState {
    pub padding_for_borsh: [u8; 3],
    /// the node that votes in this account
    pub node_pubkey: Pubkey,

    /// the signer for withdrawals
    pub authorized_withdrawer: Pubkey,
    /// percentage (0-100) that represents what part of a rewards
    ///  payout should be given to this VoteAccount
    pub commission: u8,

    pub votes: VecDeque<Lockout>,

    // This usually the last Lockout which was popped from self.votes.
    // However, it can be arbitrary slot, when being used inside Tower
    pub root_slot: Option<Slot>,

    /// the signer for vote transactions
    pub authorized_voters: AuthorizedVoters,
    // OTHER FIELDS OMITTED INORDER TO DESERIALIZE ON THE STACK.
}
impl VoteState {
    pub fn space() -> usize {
        3731
    }
    pub fn min_lamports() -> u64 {
        Rent::get().unwrap().minimum_balance(Self::space())
    }
    pub fn deserialize(input: &[u8]) -> Box<Self> {
        let collected: Box<VoteStateVersions> = try_from_slice_unchecked(input).unwrap();
        collected.convert_to_current()
    }
}

#[derive(Debug, Default, BorshDeserialize, PartialEq, Eq, Clone)]
pub struct AuthorizedVoters {
    pub authorized_voters: BTreeMap<Epoch, Pubkey>,
}
impl AuthorizedVoters {
    pub fn new(epoch: Epoch, pubkey: Pubkey) -> Self {
        let mut authorized_voters = BTreeMap::new();
        authorized_voters.insert(epoch, pubkey);
        Self { authorized_voters }
    }
    pub fn last(&self) -> Option<(&u64, &Pubkey)> {
        self.authorized_voters.iter().next_back()
    }
}

#[derive(Default, BorshDeserialize, Debug, PartialEq, Eq, Copy, Clone)]
pub struct Lockout {
    pub slot: Slot,
    pub confirmation_count: u32,
}

#[derive(BorshDeserialize, Clone)]
pub enum VoteStateVersions {
    V0_23_5(Box<VoteState0_23_5>),
    Current(Box<VoteState>),
}

impl VoteStateVersions {
    pub fn convert_to_current(self) -> Box<VoteState> {
        match self {
            VoteStateVersions::V0_23_5(state) => {
                let authorized_voters =
                    AuthorizedVoters::new(state.authorized_voter_epoch, state.authorized_voter);

                Box::new(VoteState {
                    padding_for_borsh:[0,0,0],

                    node_pubkey: state.node_pubkey,

                    /// the signer for withdrawals
                    authorized_withdrawer: state.authorized_withdrawer,

                    /// percentage (0-100) that represents what part of a rewards
                    ///  payout should be given to this VoteAccount
                    commission: state.commission,

                    votes: VecDeque::new(),

                    root_slot: None,

                    /// the signer for vote transactions
                    authorized_voters,
                })
            }
            VoteStateVersions::Current(state) => state,
        }
    }
}

#[derive(Debug, Default, BorshDeserialize, PartialEq, Eq, Clone)]
pub struct VoteState0_23_5 {
    pub padding_for_borsh: [u8; 3],
    /// the node that votes in this account
    pub node_pubkey: Pubkey,

    /// the signer for vote transactions
    pub authorized_voter: Pubkey,
    /// when the authorized voter was set/initialized
    pub authorized_voter_epoch: Epoch,

    /// history of prior authorized voters and the epoch ranges for which
    ///  they were set
    pub prior_voters: CircBuf<(Pubkey, Epoch, Epoch, Slot)>,

    /// the signer for withdrawals
    pub authorized_withdrawer: Pubkey,
    /// percentage (0-100) that represents what part of a rewards
    ///  payout should be given to this VoteAccount
    pub commission: u8,
    // OTHER FIELDS OMITTED INORDER TO DESERIALIZE ON THE STACK.
}

pub type LogLevel = u8;

const MAX_ITEMS: usize = 32;
#[derive(Debug, BorshDeserialize, PartialEq, Eq, Clone)]
pub struct CircBuf<I> {
    pub buf: [I; MAX_ITEMS],
    /// next pointer
    pub idx: usize,
}
impl<I: Default + Copy> Default for CircBuf<I> {
    fn default() -> Self {
        Self {
            buf: [I::default(); MAX_ITEMS],
            idx: MAX_ITEMS - 1,
        }
    }
}

impl<I> CircBuf<I> {
    pub fn append(&mut self, item: I) {
        // remember prior delegate and when we switched, to support later slashing
        self.idx += 1;
        self.idx %= MAX_ITEMS;

        self.buf[self.idx] = item;
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    #[test]
    fn test_escrow_and_team_fee() {
        assert!(consts::ESCROWED_BASIS_POINTS + consts::TEAM_FEES_BASIS_POINTS <= 10000)
    }
}
