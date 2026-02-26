use solana_program::program_error::ProgramError;
use thiserror::Error;

#[derive(Debug, Error, Clone, Copy)]
pub enum GatewayError {
    #[error("invalid instruction")]
    InvalidInstruction = 0,
    #[error("invalid account")]
    InvalidAccount = 1,
    #[error("unauthorized")]
    Unauthorized = 2,
    #[error("rate limited")]
    RateLimited = 3,
    #[error("quota exceeded")]
    QuotaExceeded = 4,
    #[error("insufficient prepaid balance")]
    InsufficientBalance = 5,
    #[error("key mismatch")]
    ApiKeyMismatch = 6,
    #[error("already initialized")]
    AlreadyInitialized = 7,
}

impl From<GatewayError> for ProgramError {
    fn from(value: GatewayError) -> Self {
        ProgramError::Custom(value as u32)
    }
}
