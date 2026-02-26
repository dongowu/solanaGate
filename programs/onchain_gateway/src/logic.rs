#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BucketState {
    pub capacity: u64,
    pub tokens: u64,
    pub refill_per_second: u64,
    pub last_refill_ts: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuotaState {
    pub period_seconds: i64,
    pub period_start_ts: i64,
    pub period_limit: u64,
    pub remaining: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GatewayRules {
    pub base_price_lamports: u64,
    pub max_surge_bps: u16,
    pub period_limit: u64,
    pub period_seconds: i64,
    pub bucket_capacity: u64,
    pub refill_per_second: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConsumerRuntimeState {
    pub bucket_tokens: u64,
    pub bucket_last_refill_ts: i64,
    pub quota_remaining: u64,
    pub quota_period_start_ts: i64,
    pub total_calls: u64,
    pub total_spent_lamports: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsumeError {
    RateLimited,
    QuotaExceeded,
    InsufficientBalance,
}

pub fn refill_bucket(bucket: &mut BucketState, now_ts: i64) {
    if now_ts <= bucket.last_refill_ts {
        return;
    }

    let elapsed = (now_ts - bucket.last_refill_ts) as u64;
    let refill = elapsed.saturating_mul(bucket.refill_per_second);
    bucket.tokens = bucket.capacity.min(bucket.tokens.saturating_add(refill));
    bucket.last_refill_ts = now_ts;
}

pub fn enforce_quota_window(quota: &mut QuotaState, now_ts: i64) {
    if quota.period_seconds <= 0 {
        return;
    }

    if now_ts - quota.period_start_ts >= quota.period_seconds {
        quota.period_start_ts = now_ts;
        quota.remaining = quota.period_limit;
    }
}

pub fn dynamic_price_lamports(
    base_price_lamports: u64,
    period_limit: u64,
    remaining_quota: u64,
    max_surge_bps: u16,
) -> u64 {
    if period_limit == 0 || base_price_lamports == 0 {
        return base_price_lamports;
    }

    let capped_remaining = remaining_quota.min(period_limit);
    let used = period_limit.saturating_sub(capped_remaining);
    let utilization_bps = used.saturating_mul(10_000) / period_limit;
    let surge_bps = utilization_bps.saturating_mul(max_surge_bps as u64) / 10_000;

    base_price_lamports.saturating_mul(10_000 + surge_bps) / 10_000
}

pub fn can_charge(available_balance: u64, minimum_rent: u64, charge_lamports: u64) -> bool {
    match minimum_rent.checked_add(charge_lamports) {
        Some(required_balance) => available_balance >= required_balance,
        None => false,
    }
}

pub fn apply_consume(
    rules: &GatewayRules,
    state: &mut ConsumerRuntimeState,
    now_ts: i64,
    available_balance: u64,
    minimum_rent: u64,
) -> Result<u64, ConsumeError> {
    let mut next_state = *state;

    if rules.bucket_capacity > 0 {
        let mut bucket = BucketState {
            capacity: rules.bucket_capacity,
            tokens: next_state.bucket_tokens,
            refill_per_second: rules.refill_per_second,
            last_refill_ts: next_state.bucket_last_refill_ts,
        };
        refill_bucket(&mut bucket, now_ts);

        if bucket.tokens == 0 {
            return Err(ConsumeError::RateLimited);
        }

        bucket.tokens = bucket.tokens.saturating_sub(1);
        next_state.bucket_tokens = bucket.tokens;
        next_state.bucket_last_refill_ts = bucket.last_refill_ts;
    }

    let mut remaining_quota_for_price = next_state.quota_remaining;

    if rules.period_limit > 0 {
        let mut quota = QuotaState {
            period_seconds: rules.period_seconds,
            period_start_ts: next_state.quota_period_start_ts,
            period_limit: rules.period_limit,
            remaining: next_state.quota_remaining,
        };

        enforce_quota_window(&mut quota, now_ts);
        if quota.remaining == 0 {
            return Err(ConsumeError::QuotaExceeded);
        }

        quota.remaining = quota.remaining.saturating_sub(1);
        remaining_quota_for_price = quota.remaining;
        next_state.quota_remaining = quota.remaining;
        next_state.quota_period_start_ts = quota.period_start_ts;
    }

    let price = dynamic_price_lamports(
        rules.base_price_lamports,
        rules.period_limit,
        remaining_quota_for_price,
        rules.max_surge_bps,
    );

    if !can_charge(available_balance, minimum_rent, price) {
        return Err(ConsumeError::InsufficientBalance);
    }

    next_state.total_calls = next_state.total_calls.saturating_add(1);
    next_state.total_spent_lamports = next_state.total_spent_lamports.saturating_add(price);
    *state = next_state;

    Ok(price)
}
