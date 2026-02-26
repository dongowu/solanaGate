# SolaGate: On-Chain API Gateway (Kong/Apigee on Solana)

A Solana Rust program that rebuilds key API gateway backend logic on-chain:

- API key management (hashed API credential bound to on-chain account)
- Quota window enforcement (period quota)
- Near-real-time rate limiting (token bucket)
- Prepaid pay-as-you-go billing (lamports stored in PDA and auto-debited)
- Dynamic pricing (price increases with quota utilization)

This project targets the challenge theme: **"Rebuild Backend Systems as On-Chain Rust Programs"**.

---

## 1) Web2 Pattern vs Solana Pattern

### Web2 (Kong / Apigee style)

In a traditional API gateway:

1. Client sends API key.
2. Gateway validates key from DB/cache.
3. Gateway checks rate limit + quota counters.
4. Gateway charges account usage (billing pipeline).
5. Request is forwarded to backend service.

State lives in centralized systems (Redis, SQL, internal billing DB).

### Solana (this project)

State is moved into on-chain accounts:

- `GatewayConfig` PDA = policy/config store (price, quota, bucket params, authority)
- `ConsumerAccount` PDA = API key identity + runtime counters + prepaid balance holder

A backend signer calls `Consume` before serving a request. The program atomically:

1. Refills token bucket by time elapsed.
2. Rolls quota window if period elapsed.
3. Computes dynamic price.
4. Verifies prepaid balance (respecting rent floor).
5. Debits lamports from consumer PDA to treasury.
6. Commits counters (`total_calls`, quota/tokens, `total_spent_lamports`).

---

## 2) Account Model

### `GatewayConfig` PDA
Seeds: `["gateway", admin_pubkey]`

Fields:

- `admin`
- `treasury`
- `backend_signer`
- `base_price_lamports`
- `max_surge_bps`
- `period_limit`
- `period_seconds`
- `bucket_capacity`
- `refill_per_second`

### `ConsumerAccount` PDA
Seeds: `["consumer", gateway_pubkey, owner_pubkey, api_key_id_le_bytes]`

Fields:

- `owner`
- `gateway`
- `api_key_id`
- `api_key_hash` (SHA-256 hash of API key string)
- runtime counters (bucket/quota + cumulative usage)

The consumer PDA is also the **prepaid balance vault** (lamports).

---

## 3) Instruction Set

- `InitializeGateway`
  - Creates and initializes gateway config PDA.
- `RegisterConsumer`
  - Creates consumer PDA and stores API hash + counter baseline.
- `TopUp`
  - Transfers lamports from owner wallet to consumer PDA.
- `Consume`
  - Called by backend signer to enforce limits and charge usage.

---

## 4) Dynamic Pricing + Rate Limiting

### Token Bucket

- Refill: `tokens += elapsed_seconds * refill_per_second`
- Cap: `tokens <= bucket_capacity`
- Each consume burns one token.

### Quota Window

- When `now - period_start >= period_seconds`, quota resets to `period_limit`.

### Price Function

`price = base_price * (1 + surge)`

Where surge is proportional to quota utilization, bounded by `max_surge_bps`:

- utilization = `(period_limit - remaining_quota) / period_limit`
- surge_bps = `utilization * max_surge_bps`

So remaining quota drops => price increases.

---

## 5) Rust Workspace Layout

- `programs/onchain_gateway` - on-chain program (native Solana SDK)
- `clients/gateway-cli` - minimal public CLI client
- `docs/plans/2026-02-25-onchain-api-gateway-implementation.md` - implementation plan

---

## 6) Local Testing

```bash
cargo test --offline -p onchain_gateway -- --nocapture
cargo test --offline -p gateway-cli -- --nocapture
```

Current coverage includes:

- token bucket refill + cap
- quota rollover
- dynamic price behavior
- rent-floor charging guard
- failed charge state rollback
- instruction serialization + PDA determinism
- CLI parsing + deterministic API-key hashing

---

## 7) Build

```bash
cargo build --workspace --offline
```

For SBF deployment build (requires Solana toolchain):

```bash
cargo build-sbf --manifest-path programs/onchain_gateway/Cargo.toml --no-default-features
```

---

## 8) CLI Usage (Public Test Client)

### Derive PDAs

```bash
cargo run -p gateway-cli -- \
  --program-id <PROGRAM_ID> \
  --keypair ~/.config/solana/id.json \
  derive-gateway <ADMIN_PUBKEY>

cargo run -p gateway-cli -- \
  --program-id <PROGRAM_ID> \
  --keypair ~/.config/solana/id.json \
  derive-consumer <GATEWAY_PUBKEY> <OWNER_PUBKEY> <API_KEY_ID>
```

### Initialize gateway

```bash
cargo run -p gateway-cli -- \
  --rpc-url https://api.devnet.solana.com \
  --program-id <PROGRAM_ID> \
  --keypair ~/.config/solana/admin.json \
  init-gateway \
  <TREASURY_PUBKEY> <BACKEND_SIGNER_PUBKEY> \
  10000 5000 1000 60 20 5
```

### Register consumer + top up

```bash
cargo run -p gateway-cli -- \
  --rpc-url https://api.devnet.solana.com \
  --program-id <PROGRAM_ID> \
  --keypair ~/.config/solana/user.json \
  register-consumer <GATEWAY_PUBKEY> 1 "my-secret-api-key"

cargo run -p gateway-cli -- \
  --rpc-url https://api.devnet.solana.com \
  --program-id <PROGRAM_ID> \
  --keypair ~/.config/solana/user.json \
  topup <CONSUMER_PDA> 50000000
```

### Consume (backend signer)

```bash
cargo run -p gateway-cli -- \
  --rpc-url https://api.devnet.solana.com \
  --program-id <PROGRAM_ID> \
  --keypair ~/.config/solana/backend.json \
  consume <GATEWAY_PUBKEY> <CONSUMER_PDA> <TREASURY_PUBKEY> 1 "my-secret-api-key"
```

CLI prints transaction signature and explorer URL.

---

## 9) Devnet Deployment & Evidence

- Program ID: `8EUfXTtEjzZrmtoskhCz8KR82JkgyWoBftRiDd5tRc75`
- Repo URL: `https://github.com/dongowu/solana`
- Program deploy tx: `https://explorer.solana.com/tx/5oo1HjYw3G8ALqJituiPjBtyic3x6n3vC2cbVSin9fRJRf9kvdUJMPhniAioLpG3JTyr9dcRxtidX8WYKHTQm6yH?cluster=devnet`

### Devnet transaction links

- Initialize gateway: `https://explorer.solana.com/tx/3UbntjryLfZFJcPYxvpZkpXANg1j1MqHzEAza3qo6nVqFwMKgyjUf83H2XxG9Mcs5ELYVFr1emSpYdt9VRTgdhYD?cluster=devnet`
- Register consumer: `https://explorer.solana.com/tx/2y2MZpbKjpTeay4kgSZVmno2gh5uu7fQa52Z2yGFoTvNys8a6ethjWYZMfsos6Mt3N3PeNowHVhKFd3CfRVGSVfV?cluster=devnet`
- Top up: `https://explorer.solana.com/tx/2ejiadqKRVrT6Z2b6dmppdgHQoxpihd1hy6oziy4zsyLBpLNydmVZgFTdGjpXDFnbNgpGDq1cVsdFG79D4q17YmC?cluster=devnet`
- Consume #1: `https://explorer.solana.com/tx/5NobthQL6ZGRdGBcg1VXaaXVkh9865CDBGdTmwpBA6gCCbLnD7UN2gKQdGshWgsP2pCxVySye4Y46Cnv9b178sP8?cluster=devnet`
- Consume rate-limit rejection example: preflight simulation rejected with `custom program error: 0x3` (rate limit), so no on-chain tx signature is produced.

---

## 10) Tradeoffs & Constraints

- On-chain storage/rent costs replace centralized DB costs.
- `Consume` requires backend signer: practical for gateway trust model, but not fully trustless traffic origination.
- Dynamic pricing here is quota-utilization based (deterministic on-chain). Congestion-oracle pricing is possible but needs external data feed.
- Consumer PDA holds funds directly; this is simple and auditable, but operationally requires careful rent-floor management.
- Rate limiting is near real-time by Solana slot timing, but exact wall-clock behavior depends on cluster timing jitter.

---

## 11) Challenge Checklist Mapping

- Rust on-chain program: ✅
- Architecture/account modeling explanation: ✅
- Web2 → Solana analysis + tradeoffs: ✅
- Public testable client (CLI): ✅
- Tests: ✅
- Devnet deploy + tx links: 🚧 (run section 8/9 with your wallet to finalize evidence)
