#![allow(deprecated)]

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction, system_program,
    sysvar::Sysvar,
};

use crate::{
    error::GatewayError,
    instruction::GatewayInstruction,
    logic::{apply_consume, ConsumeError, ConsumerRuntimeState, GatewayRules},
    state::{consumer_pda, gateway_pda, ConsumerAccount, GatewayConfig},
};

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = GatewayInstruction::unpack(instruction_data)
        .map_err(|_| GatewayError::InvalidInstruction)?;

    match instruction {
        GatewayInstruction::InitializeGateway {
            base_price_lamports,
            max_surge_bps,
            period_limit,
            period_seconds,
            bucket_capacity,
            refill_per_second,
        } => process_initialize_gateway(
            program_id,
            accounts,
            base_price_lamports,
            max_surge_bps,
            period_limit,
            period_seconds,
            bucket_capacity,
            refill_per_second,
        ),
        GatewayInstruction::RegisterConsumer {
            api_key_id,
            api_key_hash,
        } => process_register_consumer(program_id, accounts, api_key_id, api_key_hash),
        GatewayInstruction::TopUp { lamports } => process_topup(accounts, lamports),
        GatewayInstruction::Consume {
            api_key_id,
            presented_api_key_hash,
        } => process_consume(accounts, api_key_id, presented_api_key_hash),
    }
}

#[allow(clippy::too_many_arguments)]
fn process_initialize_gateway(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    base_price_lamports: u64,
    max_surge_bps: u16,
    period_limit: u64,
    period_seconds: i64,
    bucket_capacity: u64,
    refill_per_second: u64,
) -> ProgramResult {
    let mut iter = accounts.iter();
    let admin = next_account_info(&mut iter)?;
    let gateway_account = next_account_info(&mut iter)?;
    let treasury = next_account_info(&mut iter)?;
    let backend_signer = next_account_info(&mut iter)?;
    let system_program_account = next_account_info(&mut iter)?;

    require_signer(admin)?;
    require_writable(gateway_account)?;

    if *system_program_account.key != system_program::ID {
        return Err(GatewayError::InvalidAccount.into());
    }

    let (expected_gateway, bump) = gateway_pda(admin.key, program_id);
    if expected_gateway != *gateway_account.key {
        return Err(GatewayError::InvalidAccount.into());
    }

    create_pda_account(
        admin,
        gateway_account,
        system_program_account,
        program_id,
        &[b"gateway", admin.key.as_ref(), &[bump]],
        GatewayConfig::LEN,
    )?;

    let existing = read_gateway(gateway_account)?;
    if existing.is_initialized {
        return Err(GatewayError::AlreadyInitialized.into());
    }

    let cfg = GatewayConfig {
        is_initialized: true,
        admin: *admin.key,
        treasury: *treasury.key,
        backend_signer: *backend_signer.key,
        base_price_lamports,
        max_surge_bps,
        period_limit,
        period_seconds,
        bucket_capacity,
        refill_per_second,
        bump,
    };

    write_gateway(gateway_account, &cfg)?;
    msg!("gateway initialized");
    Ok(())
}

fn process_register_consumer(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    api_key_id: u64,
    api_key_hash: [u8; 32],
) -> ProgramResult {
    let mut iter = accounts.iter();
    let owner = next_account_info(&mut iter)?;
    let gateway_account = next_account_info(&mut iter)?;
    let consumer_account = next_account_info(&mut iter)?;
    let system_program_account = next_account_info(&mut iter)?;

    require_signer(owner)?;
    require_writable(consumer_account)?;

    if *system_program_account.key != system_program::ID {
        return Err(GatewayError::InvalidAccount.into());
    }
    if gateway_account.owner != program_id {
        return Err(GatewayError::InvalidAccount.into());
    }

    let gateway = read_gateway(gateway_account)?;
    if !gateway.is_initialized {
        return Err(GatewayError::InvalidAccount.into());
    }

    let (expected_consumer, bump) =
        consumer_pda(gateway_account.key, owner.key, api_key_id, program_id);
    if expected_consumer != *consumer_account.key {
        return Err(GatewayError::InvalidAccount.into());
    }

    create_pda_account(
        owner,
        consumer_account,
        system_program_account,
        program_id,
        &[
            b"consumer",
            gateway_account.key.as_ref(),
            owner.key.as_ref(),
            &api_key_id.to_le_bytes(),
            &[bump],
        ],
        ConsumerAccount::LEN,
    )?;

    let existing = read_consumer(consumer_account)?;
    if existing.is_initialized {
        return Err(GatewayError::AlreadyInitialized.into());
    }

    let now_ts = Clock::get()?.unix_timestamp;
    let consumer = ConsumerAccount {
        is_initialized: true,
        gateway: *gateway_account.key,
        owner: *owner.key,
        api_key_id,
        api_key_hash,
        bucket_tokens: gateway.bucket_capacity,
        bucket_last_refill_ts: now_ts,
        quota_remaining: gateway.period_limit,
        quota_period_start_ts: now_ts,
        total_calls: 0,
        total_spent_lamports: 0,
        bump,
    };

    write_consumer(consumer_account, &consumer)?;
    msg!("consumer registered");
    Ok(())
}

fn process_topup(accounts: &[AccountInfo], lamports: u64) -> ProgramResult {
    let mut iter = accounts.iter();
    let owner = next_account_info(&mut iter)?;
    let consumer_account = next_account_info(&mut iter)?;
    let system_program_account = next_account_info(&mut iter)?;

    require_signer(owner)?;

    if *system_program_account.key != system_program::ID {
        return Err(GatewayError::InvalidAccount.into());
    }

    let consumer = read_consumer(consumer_account)?;
    if !consumer.is_initialized || consumer.owner != *owner.key {
        return Err(GatewayError::Unauthorized.into());
    }

    invoke(
        &system_instruction::transfer(owner.key, consumer_account.key, lamports),
        &[
            owner.clone(),
            consumer_account.clone(),
            system_program_account.clone(),
        ],
    )?;

    Ok(())
}

fn process_consume(
    accounts: &[AccountInfo],
    api_key_id: u64,
    presented_api_key_hash: [u8; 32],
) -> ProgramResult {
    let mut iter = accounts.iter();
    let backend = next_account_info(&mut iter)?;
    let gateway_account = next_account_info(&mut iter)?;
    let consumer_account = next_account_info(&mut iter)?;
    let treasury_account = next_account_info(&mut iter)?;

    require_signer(backend)?;
    require_writable(consumer_account)?;
    require_writable(treasury_account)?;

    let gateway = read_gateway(gateway_account)?;
    if !gateway.is_initialized {
        return Err(GatewayError::InvalidAccount.into());
    }
    if gateway.backend_signer != *backend.key {
        return Err(GatewayError::Unauthorized.into());
    }
    if gateway.treasury != *treasury_account.key {
        return Err(GatewayError::InvalidAccount.into());
    }

    let mut consumer = read_consumer(consumer_account)?;
    if !consumer.is_initialized {
        return Err(GatewayError::InvalidAccount.into());
    }
    if consumer.gateway != *gateway_account.key {
        return Err(GatewayError::InvalidAccount.into());
    }
    if consumer.api_key_id != api_key_id || consumer.api_key_hash != presented_api_key_hash {
        return Err(GatewayError::ApiKeyMismatch.into());
    }

    let rules = GatewayRules {
        base_price_lamports: gateway.base_price_lamports,
        max_surge_bps: gateway.max_surge_bps,
        period_limit: gateway.period_limit,
        period_seconds: gateway.period_seconds,
        bucket_capacity: gateway.bucket_capacity,
        refill_per_second: gateway.refill_per_second,
    };

    let mut runtime = ConsumerRuntimeState {
        bucket_tokens: consumer.bucket_tokens,
        bucket_last_refill_ts: consumer.bucket_last_refill_ts,
        quota_remaining: consumer.quota_remaining,
        quota_period_start_ts: consumer.quota_period_start_ts,
        total_calls: consumer.total_calls,
        total_spent_lamports: consumer.total_spent_lamports,
    };

    let now_ts = Clock::get()?.unix_timestamp;
    let available_balance = **consumer_account.lamports.borrow();
    let minimum_rent = Rent::get()?.minimum_balance(ConsumerAccount::LEN);

    let charge = apply_consume(
        &rules,
        &mut runtime,
        now_ts,
        available_balance,
        minimum_rent,
    )
    .map_err(map_consume_error)?;

    {
        let mut source = consumer_account.try_borrow_mut_lamports()?;
        if **source < charge {
            return Err(GatewayError::InsufficientBalance.into());
        }
        **source -= charge;
    }

    {
        let mut dest = treasury_account.try_borrow_mut_lamports()?;
        **dest = (**dest)
            .checked_add(charge)
            .ok_or(ProgramError::ArithmeticOverflow)?;
    }

    consumer.bucket_tokens = runtime.bucket_tokens;
    consumer.bucket_last_refill_ts = runtime.bucket_last_refill_ts;
    consumer.quota_remaining = runtime.quota_remaining;
    consumer.quota_period_start_ts = runtime.quota_period_start_ts;
    consumer.total_calls = runtime.total_calls;
    consumer.total_spent_lamports = runtime.total_spent_lamports;

    write_consumer(consumer_account, &consumer)?;
    Ok(())
}

fn create_pda_account<'a>(
    payer: &AccountInfo<'a>,
    pda: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    program_id: &Pubkey,
    signer_seeds: &[&[u8]],
    data_len: usize,
) -> ProgramResult {
    if pda.owner == program_id && pda.data_len() == data_len {
        return Ok(());
    }

    if pda.owner != &system_program::ID {
        return Err(GatewayError::InvalidAccount.into());
    }

    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(data_len);

    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            pda.key,
            lamports,
            data_len as u64,
            program_id,
        ),
        &[payer.clone(), pda.clone(), system_program_account.clone()],
        &[signer_seeds],
    )
}

fn read_gateway(account: &AccountInfo) -> Result<GatewayConfig, ProgramError> {
    if account.data_len() != GatewayConfig::LEN {
        return Err(GatewayError::InvalidAccount.into());
    }
    GatewayConfig::try_from_slice(&account.try_borrow_data()?)
        .map_err(|_| ProgramError::InvalidAccountData)
}

fn write_gateway(account: &AccountInfo, cfg: &GatewayConfig) -> ProgramResult {
    let mut data = account.try_borrow_mut_data()?;
    cfg.serialize(&mut &mut data[..])
        .map_err(|_| ProgramError::InvalidAccountData)
}

fn read_consumer(account: &AccountInfo) -> Result<ConsumerAccount, ProgramError> {
    if account.data_len() != ConsumerAccount::LEN {
        return Err(GatewayError::InvalidAccount.into());
    }
    ConsumerAccount::try_from_slice(&account.try_borrow_data()?)
        .map_err(|_| ProgramError::InvalidAccountData)
}

fn write_consumer(account: &AccountInfo, consumer: &ConsumerAccount) -> ProgramResult {
    let mut data = account.try_borrow_mut_data()?;
    consumer
        .serialize(&mut &mut data[..])
        .map_err(|_| ProgramError::InvalidAccountData)
}

fn map_consume_error(err: ConsumeError) -> ProgramError {
    match err {
        ConsumeError::RateLimited => GatewayError::RateLimited.into(),
        ConsumeError::QuotaExceeded => GatewayError::QuotaExceeded.into(),
        ConsumeError::InsufficientBalance => GatewayError::InsufficientBalance.into(),
    }
}

fn require_signer(account: &AccountInfo) -> ProgramResult {
    if !account.is_signer {
        return Err(GatewayError::Unauthorized.into());
    }
    Ok(())
}

fn require_writable(account: &AccountInfo) -> ProgramResult {
    if !account.is_writable {
        return Err(GatewayError::InvalidAccount.into());
    }
    Ok(())
}
