use borsh::{BorshDeserialize, BorshSerialize};

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub enum GatewayInstruction {
    InitializeGateway {
        base_price_lamports: u64,
        max_surge_bps: u16,
        period_limit: u64,
        period_seconds: i64,
        bucket_capacity: u64,
        refill_per_second: u64,
    },
    RegisterConsumer {
        api_key_id: u64,
        api_key_hash: [u8; 32],
    },
    TopUp {
        lamports: u64,
    },
    Consume {
        api_key_id: u64,
        presented_api_key_hash: [u8; 32],
    },
}

impl GatewayInstruction {
    pub fn unpack(input: &[u8]) -> Result<Self, std::io::Error> {
        GatewayInstruction::try_from_slice(input)
    }

    pub fn pack(&self) -> Result<Vec<u8>, std::io::Error> {
        borsh::to_vec(self)
    }
}
