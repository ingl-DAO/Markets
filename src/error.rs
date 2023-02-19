use crate::{
    colored_log,
    state::LogColors::{Blank, Blue, Green, Red},
};
use ingl_macros::InglErr;
use solana_program::program_error::ProgramError;

const LOG_LEVEL: u8 = 5;
#[derive(InglErr, Debug)]
pub enum InglError {
    #[err("Provided address is dissimilar from the expected one")]
    AddressMismatch,

    #[err("Provided Struct Type does not match expected value.")]
    InvalidStructType,

    #[err("Executing a process earlier than is allowed.")]
    TooEarly,

    #[err("Executing a process later than is allowed.")]
    TooLate,

    #[err("A certain operation yielded a value beyond bounds.")]
    BeyondBounds,

    #[err("Validation Phrase Found in the sent account is different from that expected.")]
    InvalidValPhrase,

    #[err("The account type must be a buffer, a delineation exists between the sent type and the expected type.")]
    ExpectedBufferAccount,

    #[err("An Error Occured while unwrapping an Option")]
    OptionUnwrapError,

    #[err("Invalid Data")]
    InvalidData,
}
