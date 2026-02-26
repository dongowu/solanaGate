#![allow(deprecated)]

use std::error::Error;

use clap::{Parser, Subcommand};
use solagate::{
    instruction::GatewayInstruction,
    state::{consumer_pda, gateway_pda},
};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    hash::hash,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair, Signer},
    system_program,
    transaction::Transaction,
};

#[derive(Debug, Parser)]
#[command(
    name = "solagate-cli",
    about = "CLI for SolaGate (on-chain API quota/billing gateway)"
)]
struct Cli {
    #[arg(long, default_value = "https://api.devnet.solana.com")]
    rpc_url: String,
    #[arg(long)]
    program_id: Pubkey,
    #[arg(long)]
    keypair: String,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    DeriveGateway {
        admin: Pubkey,
    },
    DeriveConsumer {
        gateway: Pubkey,
        owner: Pubkey,
        api_key_id: u64,
    },
    InitGateway {
        treasury: Pubkey,
        backend_signer: Pubkey,
        base_price_lamports: u64,
        max_surge_bps: u16,
        period_limit: u64,
        period_seconds: i64,
        bucket_capacity: u64,
        refill_per_second: u64,
    },
    RegisterConsumer {
        gateway: Pubkey,
        api_key_id: u64,
        api_key: String,
    },
    Topup {
        consumer: Pubkey,
        lamports: u64,
    },
    Consume {
        gateway: Pubkey,
        consumer: Pubkey,
        treasury: Pubkey,
        api_key_id: u64,
        api_key: String,
    },
}

fn run(cli: Cli) -> Result<(), Box<dyn Error>> {
    match cli.command {
        Commands::DeriveGateway { admin } => {
            let (gateway, bump) = gateway_pda(&admin, &cli.program_id);
            println!("gateway_pda={gateway}");
            println!("bump={bump}");
            Ok(())
        }
        Commands::DeriveConsumer {
            gateway,
            owner,
            api_key_id,
        } => {
            let (consumer, bump) = consumer_pda(&gateway, &owner, api_key_id, &cli.program_id);
            println!("consumer_pda={consumer}");
            println!("bump={bump}");
            Ok(())
        }
        other => {
            let signer = read_keypair_file(&cli.keypair)
                .map_err(|e| format!("failed to read keypair file {}: {e}", cli.keypair))?;
            run_online_command(&cli.rpc_url, cli.program_id, &signer, other)
        }
    }
}

fn run_online_command(
    rpc_url: &str,
    program_id: Pubkey,
    signer: &Keypair,
    command: Commands,
) -> Result<(), Box<dyn Error>> {
    let rpc = RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed());

    let ix = match command {
        Commands::InitGateway {
            treasury,
            backend_signer,
            base_price_lamports,
            max_surge_bps,
            period_limit,
            period_seconds,
            bucket_capacity,
            refill_per_second,
        } => {
            let (gateway, _) = gateway_pda(&signer.pubkey(), &program_id);
            let data = GatewayInstruction::InitializeGateway {
                base_price_lamports,
                max_surge_bps,
                period_limit,
                period_seconds,
                bucket_capacity,
                refill_per_second,
            }
            .pack()?;

            Instruction {
                program_id,
                accounts: vec![
                    AccountMeta::new(signer.pubkey(), true),
                    AccountMeta::new(gateway, false),
                    AccountMeta::new_readonly(treasury, false),
                    AccountMeta::new_readonly(backend_signer, false),
                    AccountMeta::new_readonly(system_program::id(), false),
                ],
                data,
            }
        }
        Commands::RegisterConsumer {
            gateway,
            api_key_id,
            api_key,
        } => {
            let (consumer, _) = consumer_pda(&gateway, &signer.pubkey(), api_key_id, &program_id);
            let data = GatewayInstruction::RegisterConsumer {
                api_key_id,
                api_key_hash: api_key_hash(&api_key),
            }
            .pack()?;

            Instruction {
                program_id,
                accounts: vec![
                    AccountMeta::new(signer.pubkey(), true),
                    AccountMeta::new_readonly(gateway, false),
                    AccountMeta::new(consumer, false),
                    AccountMeta::new_readonly(system_program::id(), false),
                ],
                data,
            }
        }
        Commands::Topup { consumer, lamports } => {
            let data = GatewayInstruction::TopUp { lamports }.pack()?;
            Instruction {
                program_id,
                accounts: vec![
                    AccountMeta::new(signer.pubkey(), true),
                    AccountMeta::new(consumer, false),
                    AccountMeta::new_readonly(system_program::id(), false),
                ],
                data,
            }
        }
        Commands::Consume {
            gateway,
            consumer,
            treasury,
            api_key_id,
            api_key,
        } => {
            let data = GatewayInstruction::Consume {
                api_key_id,
                presented_api_key_hash: api_key_hash(&api_key),
            }
            .pack()?;

            Instruction {
                program_id,
                accounts: vec![
                    AccountMeta::new_readonly(signer.pubkey(), true),
                    AccountMeta::new_readonly(gateway, false),
                    AccountMeta::new(consumer, false),
                    AccountMeta::new(treasury, false),
                ],
                data,
            }
        }
        Commands::DeriveGateway { .. } | Commands::DeriveConsumer { .. } => {
            return Err("internal error: derive command routed to online path".into());
        }
    };

    let sig = send_instruction(&rpc, signer, ix)?;
    println!("signature={sig}");
    println!("explorer=https://explorer.solana.com/tx/{sig}?cluster=devnet");
    Ok(())
}

fn send_instruction(
    rpc: &RpcClient,
    signer: &Keypair,
    instruction: Instruction,
) -> Result<solana_sdk::signature::Signature, Box<dyn Error>> {
    let recent_blockhash = rpc.get_latest_blockhash()?;
    let tx = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&signer.pubkey()),
        &[signer],
        recent_blockhash,
    );

    let sig = rpc.send_and_confirm_transaction(&tx)?;
    Ok(sig)
}

fn api_key_hash(input: &str) -> [u8; 32] {
    hash(input.as_bytes()).to_bytes()
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    run(cli)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_derive_consumer_command() {
        let gateway = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();

        let cli = Cli::parse_from([
            "solagate-cli",
            "--program-id",
            &program_id.to_string(),
            "--keypair",
            "/tmp/dummy.json",
            "derive-consumer",
            &gateway.to_string(),
            &owner.to_string(),
            "7",
        ]);

        match cli.command {
            Commands::DeriveConsumer {
                gateway: parsed_gateway,
                owner: parsed_owner,
                api_key_id,
            } => {
                assert_eq!(parsed_gateway, gateway);
                assert_eq!(parsed_owner, owner);
                assert_eq!(api_key_id, 7);
            }
            _ => panic!("wrong command variant"),
        }
    }

    #[test]
    fn run_derive_gateway_returns_ok() {
        let program_id = Pubkey::new_unique();
        let admin = Pubkey::new_unique();

        let cli = Cli {
            rpc_url: "https://api.devnet.solana.com".into(),
            program_id,
            keypair: "/tmp/dummy.json".into(),
            command: Commands::DeriveGateway { admin },
        };

        let result = run(cli);
        assert!(result.is_ok());
    }

    #[test]
    fn api_key_hash_is_deterministic() {
        assert_eq!(api_key_hash("abc"), api_key_hash("abc"));
        assert_ne!(api_key_hash("abc"), api_key_hash("def"));
    }
}
