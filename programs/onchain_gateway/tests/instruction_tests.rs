use onchain_gateway::{
    instruction::GatewayInstruction, state::consumer_pda, state::gateway_pda, ID,
};
use solana_sdk::pubkey::Pubkey;

#[test]
fn instruction_roundtrip_works() {
    let ix = GatewayInstruction::RegisterConsumer {
        api_key_id: 42,
        api_key_hash: [7u8; 32],
    };

    let encoded = ix.pack().expect("serialize");
    let decoded = GatewayInstruction::unpack(&encoded).expect("deserialize");

    assert_eq!(decoded, ix);
}

#[test]
fn pda_derivation_is_deterministic() {
    let admin = Pubkey::new_unique();
    let owner = Pubkey::new_unique();
    let (gateway, _) = gateway_pda(&admin, &ID);

    let (consumer_a, _) = consumer_pda(&gateway, &owner, 11, &ID);
    let (consumer_b, _) = consumer_pda(&gateway, &owner, 11, &ID);
    let (consumer_c, _) = consumer_pda(&gateway, &owner, 12, &ID);

    assert_eq!(consumer_a, consumer_b);
    assert_ne!(consumer_a, consumer_c);
}
