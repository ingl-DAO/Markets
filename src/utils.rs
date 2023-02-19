use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::{self, clock::Clock, rent::Rent, Sysvar},
};
use std::slice::Iter;

use crate::{colored_log, error::InglError, state::LogColors::*};
pub trait PubkeyHelpers {
    fn assert_match(&self, a: &Pubkey) -> ProgramResult;
}

impl PubkeyHelpers for Pubkey {
    fn assert_match(&self, a: &Pubkey) -> ProgramResult {
        if self != a {
            let keyword = "Left: ".to_string() + &self.to_string() + ", Right: " + &a.to_string();
            Err(InglError::AddressMismatch.utilize(&keyword))?
        }
        Ok(())
    }
}

pub trait AccountInfoHelpers {
    fn assert_key_match(&self, a: &Pubkey) -> ProgramResult;
    fn assert_owner(&self, a: &Pubkey) -> ProgramResult;
    fn assert_signer(&self) -> ProgramResult;
    fn assert_seed(
        &self,
        program_id: &Pubkey,
        seed: &[&[u8]],
    ) -> Result<(Pubkey, u8), ProgramError>;
}

impl AccountInfoHelpers for AccountInfo<'_> {
    fn assert_key_match(&self, a: &Pubkey) -> ProgramResult {
        self.key.assert_match(a)
    }
    fn assert_owner(&self, a: &Pubkey) -> ProgramResult {
        self.owner
            .assert_match(a)
            .error_log("Error: @ owner assertion.")
    }
    fn assert_signer(&self) -> ProgramResult {
        if !self.is_signer {
            Err(ProgramError::MissingRequiredSignature)?
        }
        Ok(())
    }
    fn assert_seed(
        &self,
        program_id: &Pubkey,
        seed: &[&[u8]],
    ) -> Result<(Pubkey, u8), ProgramError> {
        let (key, bump) = Pubkey::find_program_address(seed, program_id);
        self.assert_key_match(&key)
            .error_log("Error: @ PDA Assertion")?;
        Ok((key, bump))
    }
}

/// Get clock_data
pub fn get_clock_data(
    iter: &mut Iter<AccountInfo>,
    clock_is_from_account: bool,
) -> Result<Clock, ProgramError> {
    (if clock_is_from_account {
        let sysvar_clock_info = iter.next().error_log("Not Enough Keys to get Clock_data")?;
        sysvar_clock_info
            .assert_key_match(&sysvar::clock::id())
            .error_log("Error: Error @ sysvar_clock_info assertion.")?;
        sysvar_clock_info
            .assert_owner(&sysvar::id())
            .error_log("Error: Error @ sysvar_clock_info ownership assertion.")?;
        Clock::from_account_info(&sysvar_clock_info)
    } else {
        Clock::get()
    })
    .error_log("Error: There are some issues getting clock details")
}

/// Get clock_data from account info
pub fn get_clock_data_from_account(sysvar_clock_info: &AccountInfo) -> Result<Clock, ProgramError> {
    sysvar_clock_info
        .assert_key_match(&sysvar::clock::id())
        .error_log("Error: Error @ sysvar_clock_info assertion.")?;
    sysvar_clock_info
        .assert_owner(&sysvar::id())
        .error_log("Error: Error @ sysvar_clock_info ownership assertion.")?;
    Clock::from_account_info(&sysvar_clock_info)
        .error_log("Error: There are some issues getting clock details")
}

/// Get rent_data
pub fn get_rent_data(
    iter: &mut Iter<AccountInfo>,
    rent_is_from_account: bool,
) -> Result<Rent, ProgramError> {
    (if rent_is_from_account {
        let sysvar_rent_info = iter.next().error_log("Not Enough Keys to get rent_data")?;
        sysvar_rent_info
            .assert_key_match(&sysvar::rent::id())
            .error_log("Error: Error @ sysvar_rent_info assertion.")?;
        sysvar_rent_info
            .assert_owner(&sysvar::id())
            .error_log("Error: Error @ sysvar_clock_info ownership assertion.")?;
        Rent::from_account_info(&sysvar_rent_info)
    } else {
        Rent::get()
    })
    .error_log("Error: There are some issues getting rent details")
}

/// Get rent_data from account info
pub fn get_rent_data_from_account(sysvar_rent_info: &AccountInfo) -> Result<Rent, ProgramError> {
    sysvar_rent_info
        .assert_key_match(&sysvar::rent::id())
        .error_log("Error: Error @ sysvar_rent_info assertion.")?;
    sysvar_rent_info
        .assert_owner(&sysvar::id())
        .error_log("Error: Error @ sysvar_clock_info ownership assertion.")?;
    Rent::from_account_info(&sysvar_rent_info)
        .error_log("Error: There are some issues getting rent details")
}

/// LEVEL 5: These logs will always run, regardless of state.rs' log level. .
/// LEVEL 4: These logs are used to log entry and exits of functions.
/// LEVEL 3: .
/// LEVEL 2: These logs to log cross program invocations and other important events, at start and end.
/// LEVEL 1: .
/// LEVEL 0: These logs are used to log the program flow at any point.
#[macro_export]
macro_rules! log {
    ($ll:expr, $log_level:expr, $msg:expr) => {
        if $log_level >= $ll || $log_level >= 5 {
            solana_program::log::sol_log($msg);
        }
    };

    ($ll:expr, $log_level:expr, $($arg:tt)*) => {
        if $log_level >= $ll || $log_level >= 5 {
            solana_program::log::sol_log(&format!($($arg)*));
        }
    };
}

#[macro_export]
macro_rules! colored_log {
        ($ll:expr, $log_level:expr, $col:expr, $msg:expr) => {
            match $col{
                Red => {
                    if $log_level >= $ll || $log_level >= 5 {
                        solana_program::log::sol_log(&format!("{}{}{}", "\x1b[31m", $msg,  "\x1b[0m"));
                    }
                }
                Green => {
                    if $log_level >= $ll || $log_level >= 5 {
                        solana_program::log::sol_log(&format!("{}{}{}", "\x1b[32m", $msg,  "\x1b[0m"));
                    }
                }
                Blue => {
                    if $log_level >= $ll || $log_level >= 5 {
                        solana_program::log::sol_log(&format!("{}{}{}", "\x1b[34m", $msg,  "\x1b[0m"));
                    }
                }

                Blank => {
                    if $log_level >= $ll || $log_level >= 5 {
                        solana_program::log::sol_log(&format!("{}{}{}", "\x1b[0m", $msg,  "\x1b[0m"));
                    }
                }

            }
        };

        ($ll:expr, $log_level:expr, $col:expr, $($arg:tt)*) => {
            match $col{
                Red => {
                    if $log_level >= $ll || $log_level >= 5 {
                        solana_program::log::sol_log(&format!("{}{}{}", "\x1b[31m", format!($($arg)*), "\x1b[0m",));
                    }
                }
                Green => {
                    if $log_level >= $ll || $log_level >= 5 {
                        solana_program::log::sol_log(&format!("{}{}{}", "\x1b[32m", format!($($arg)*), "\x1b[0m",));
                    }
                }
                Blue => {
                    if $log_level >= $ll || $log_level >= 5 {
                        solana_program::log::sol_log(&format!("{}{}{}", "\x1b[34m", format!($($arg)*), "\x1b[0m",));
                    }
                }
                Blank => {
                    if $log_level >= $ll || $log_level >= 5 {
                        solana_program::log::sol_log(&format!("{}{}{}", "\x1b[0m", format!($($arg)*), "\x1b[0m",));
                    }
                }

            }
        };
    }

pub trait ResultExt<T, E> {
    fn error_log(self, message: &str) -> Self;
}

impl<T, E> ResultExt<T, E> for Result<T, E> {
    ///Logs the error message if the result is an error, then returns the Err.
    /// If the result is an Ok(x), returns the Ok(x).
    fn error_log(self, message: &str) -> Self {
        match self {
            Ok(v) => Ok(v),
            Err(e) => {
                colored_log!(0, 5, Red, "Error: {:?}", message);
                Err(e)
            }
        }
    }
}

pub trait OptionExt<T> {
    fn error_log(self, message: &str) -> Result<T, ProgramError>;
}

impl<T> OptionExt<T> for Option<T> {
    fn error_log(self, message: &str) -> Result<T, ProgramError> {
        match self {
            Some(v) => Ok(v),
            _ => Err(InglError::OptionUnwrapError.utilize(message)),
        }
    }
}
