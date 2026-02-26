use onchain_gateway::logic::{apply_consume, ConsumeError, ConsumerRuntimeState, GatewayRules};

#[test]
fn consume_updates_counters_and_charges_balance() {
    let rules = GatewayRules {
        base_price_lamports: 1_000,
        max_surge_bps: 2_000,
        period_limit: 100,
        period_seconds: 60,
        bucket_capacity: 10,
        refill_per_second: 2,
    };

    let mut state = ConsumerRuntimeState {
        bucket_tokens: 5,
        bucket_last_refill_ts: 100,
        quota_remaining: 100,
        quota_period_start_ts: 100,
        total_calls: 0,
        total_spent_lamports: 0,
    };

    let charge = apply_consume(&rules, &mut state, 101, 5_000_000, 1_000_000).expect("consume ok");
    assert!(charge >= 1_000);
    assert_eq!(state.quota_remaining, 99);
    assert_eq!(state.total_calls, 1);
    assert_eq!(state.total_spent_lamports, charge);
}

#[test]
fn consume_fails_when_bucket_empty() {
    let rules = GatewayRules {
        base_price_lamports: 1_000,
        max_surge_bps: 2_000,
        period_limit: 100,
        period_seconds: 60,
        bucket_capacity: 1,
        refill_per_second: 0,
    };

    let mut state = ConsumerRuntimeState {
        bucket_tokens: 0,
        bucket_last_refill_ts: 100,
        quota_remaining: 100,
        quota_period_start_ts: 100,
        total_calls: 0,
        total_spent_lamports: 0,
    };

    let err = apply_consume(&rules, &mut state, 101, 5_000_000, 1_000_000)
        .expect_err("should rate limit");
    assert_eq!(err, ConsumeError::RateLimited);
}

#[test]
fn consume_fails_when_balance_below_rent_floor() {
    let rules = GatewayRules {
        base_price_lamports: 1_000,
        max_surge_bps: 2_000,
        period_limit: 100,
        period_seconds: 60,
        bucket_capacity: 10,
        refill_per_second: 0,
    };

    let mut state = ConsumerRuntimeState {
        bucket_tokens: 5,
        bucket_last_refill_ts: 100,
        quota_remaining: 100,
        quota_period_start_ts: 100,
        total_calls: 0,
        total_spent_lamports: 0,
    };

    let err = apply_consume(&rules, &mut state, 101, 1_000_100, 1_000_000)
        .expect_err("should fail rent floor");
    assert_eq!(err, ConsumeError::InsufficientBalance);
}

#[test]
fn failed_charge_does_not_consume_quota_or_tokens() {
    let rules = GatewayRules {
        base_price_lamports: 1_000,
        max_surge_bps: 2_000,
        period_limit: 100,
        period_seconds: 60,
        bucket_capacity: 10,
        refill_per_second: 0,
    };

    let mut state = ConsumerRuntimeState {
        bucket_tokens: 5,
        bucket_last_refill_ts: 100,
        quota_remaining: 100,
        quota_period_start_ts: 100,
        total_calls: 0,
        total_spent_lamports: 0,
    };

    let err =
        apply_consume(&rules, &mut state, 101, 1_000_000, 1_000_000).expect_err("should fail");
    assert_eq!(err, ConsumeError::InsufficientBalance);
    assert_eq!(state.bucket_tokens, 5);
    assert_eq!(state.quota_remaining, 100);
    assert_eq!(state.total_calls, 0);
}
