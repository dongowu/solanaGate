use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct GatewayConfig {
    pub is_initialized: bool,
    pub admin: Pubkey,
    pub treasury: Pubkey,
    pub backend_signer: Pubkey,
    pub base_price_lamports: u64,
    pub max_surge_bps: u16,
    pub period_limit: u64,
    pub period_seconds: i64,
    pub bucket_capacity: u64,
    pub refill_per_second: u64,
    pub bump: u8,
}

impl GatewayConfig {
    pub const LEN: usize = 1 + 32 + 32 + 32 + 8 + 2 + 8 + 8 + 8 + 8 + 1;
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct ConsumerAccount {
    pub is_initialized: bool,
    pub gateway: Pubkey,
    pub owner: Pubkey,
    pub api_key_id: u64,
    pub api_key_hash: [u8; 32],
    pub bucket_tokens: u64,
    pub bucket_last_refill_ts: i64,
    pub quota_remaining: u64,
    pub quota_period_start_ts: i64,
    pub total_calls: u64,
    pub total_spent_lamports: u64,
    pub bump: u8,
}

impl ConsumerAccount {
    pub const LEN: usize = 1 + 32 + 32 + 8 + 32 + 8 + 8 + 8 + 8 + 8 + 8 + 1;
}

pub fn gateway_pda(admin: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"gateway", admin.as_ref()], program_id)
}

pub fn consumer_pda(
    gateway: &Pubkey,
    owner: &Pubkey,
    api_key_id: u64,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            b"consumer",
            gateway.as_ref(),
            owner.as_ref(),
            &api_key_id.to_le_bytes(),
        ],
        program_id,
    )
}
