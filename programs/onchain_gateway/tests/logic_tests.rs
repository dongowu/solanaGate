use solagate::logic::{
    can_charge, dynamic_price_lamports, enforce_quota_window, refill_bucket, BucketState,
    QuotaState,
};

#[test]
fn bucket_refills_and_caps_at_capacity() {
    let mut bucket = BucketState {
        capacity: 10,
        tokens: 1,
        refill_per_second: 3,
        last_refill_ts: 100,
    };

    refill_bucket(&mut bucket, 103);
    assert_eq!(bucket.tokens, 10);
    assert_eq!(bucket.last_refill_ts, 103);
}

#[test]
fn quota_rolls_over_after_window() {
    let mut quota = QuotaState {
        period_seconds: 60,
        period_start_ts: 0,
        period_limit: 100,
        remaining: 0,
    };

    enforce_quota_window(&mut quota, 61);
    assert_eq!(quota.period_start_ts, 61);
    assert_eq!(quota.remaining, 100);
}

#[test]
fn dynamic_pricing_rises_with_utilization() {
    let low_util = dynamic_price_lamports(1_000, 100, 90, 5_000);
    let high_util = dynamic_price_lamports(1_000, 100, 10, 5_000);

    assert!(high_util > low_util);
    assert_eq!(dynamic_price_lamports(1_000, 100, 100, 5_000), 1_000);
}

#[test]
fn charge_checks_post_rent_balance() {
    assert!(can_charge(2_000_000, 1_000_000, 500_000));
    assert!(!can_charge(1_400_000, 1_000_000, 500_000));
}
