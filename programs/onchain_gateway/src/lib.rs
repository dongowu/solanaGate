pub mod error;
pub mod instruction;
pub mod logic;
pub mod processor;
pub mod state;

#[cfg(not(feature = "no-entrypoint"))]
use solana_program::entrypoint;
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

solana_program::declare_id!("GaTeWw4z1cR6cFT6Qx3nJQhS7r9L9G2yQ9n2hSxGtwY");

#[cfg(not(feature = "no-entrypoint"))]
entrypoint!(process_instruction);

#[cfg_attr(feature = "no-entrypoint", allow(dead_code))]
fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    processor::process_instruction(program_id, accounts, instruction_data)
}
