#![allow(unused_parens)]
use crate::{
    colored_log,
    error::InglError,
    utils::{AccountInfoHelpers, ResultExt},
};
use bincode::deserialize;
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
    pub const PDA_AUTHORIZED_WITHDRAWER_SEED: &[u8] = b"authorized_withdrawer";
    pub const PROGRAM_STORAGE_SEED: &[u8] = b"program_storage";
    pub const PDA_UPGRADE_AUTHORITY_SEED: &[u8] = b"upgrade_authority";

    pub const STORAGE_VALIDATION_PHRASE: u32 = 838_927_652;
}

const LOG_LEVEL: u8 = 5;

#[derive(BorshDeserialize, BorshSerialize, Debug, Validate)]
#[validation_phrase(crate::state::consts::STORAGE_VALIDATION_PHRASE)]
pub struct Storage {
    pub validation_phrase: u32,
    pub authorized_withdrawer: Pubkey,
    pub vote_account: Pubkey,
    pub authorized_withdrawer_cost: u64,
    pub secondary_items: Vec<StoredSecondaryItem>,
    pub description: String,
    pub purchase: Option<Purchase>,
}

impl Storage {
    pub fn get_space(&self) -> usize {
        4 + 32
            + 32
            + 8
            + 4
            + self
                .secondary_items
                .iter()
                .map(|x| x.get_space())
                .sum::<usize>()
            + 4
            + self.description.len()
            + 1
            + Purchase::get_space()
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct Purchase {
    pub buyer: Pubkey,
    pub date: u32,
}

impl Purchase {
    pub fn get_space() -> usize {
        32 + 4
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

#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct VoteInit {
    pub node_pubkey: Pubkey,
    pub authorized_voter: Pubkey,
    pub authorized_withdrawer: Pubkey,
    pub commission: u8,
}
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
pub enum VoteAuthorize {
    Voter,
    Withdrawer,
}

#[derive(Deserialize)]
pub struct VoteState {
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
        let collected = Box::new(deserialize::<VoteStateVersions>(input).unwrap());
        collected.convert_to_current()
    }
}

pub type UnixTimestamp = i64;
#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct BlockTimestamp {
    pub slot: Slot,
    pub timestamp: UnixTimestamp,
}
#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq, Clone)]
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

#[derive(Serialize, Default, Deserialize, Debug, PartialEq, Eq, Copy, Clone)]
pub struct Lockout {
    pub slot: Slot,
    pub confirmation_count: u32,
}

#[derive(Deserialize)]
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

#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct VoteState0_23_5 {
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
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
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

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
pub enum UpgradeableLoaderState {
    /// Account is not initialized.
    Uninitialized,
    /// A Buffer account.
    Buffer {
        /// Authority address
        authority_address: Option<Pubkey>,
        // The raw program data follows this serialized structure in the
        // account's data.
    },
    /// An Program account.
    Program {
        /// Address of the ProgramData account.
        programdata_address: Pubkey,
    },
    // A ProgramData account.
    ProgramData {
        /// Slot that the program was last modified.
        slot: u64,
        /// Address of the Program's upgrade authority.
        upgrade_authority_address: Option<Pubkey>, // TODO: Check that the upgrade_authority_address is a signer during intialization.
                                                   // The raw program data follows this serialized structure in the
                                                   // account's data.
    },
}
