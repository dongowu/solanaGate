# On-Chain API Gateway (Quota + Billing) Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a production-leaning Solana Rust program that reimplements API key + quota + rate limiting + prepaid billing as on-chain state, with a public CLI for Devnet testing.

**Architecture:** A native Solana program models a gateway config PDA and consumer/API-key PDA accounts. Backend signer calls `consume` to enforce token-bucket and quota-window checks, then charges prepaid lamports from the consumer PDA to a treasury account with dynamic pricing based on quota utilization.

**Tech Stack:** Rust, solana-program, borsh, solana-program-test (tests), clap + solana-client (CLI).

---

### Task 1: Workspace and failing logic tests

**Files:**
- Create: `Cargo.toml`
- Create: `programs/onchain_gateway/Cargo.toml`
- Create: `programs/onchain_gateway/src/lib.rs`
- Create: `programs/onchain_gateway/src/logic.rs`
- Create: `programs/onchain_gateway/tests/logic_tests.rs`

**Step 1: Write failing tests**
- Add tests for:
  - token bucket refill and depletion
  - quota window rollover reset
  - dynamic price increases with utilization
  - insufficient balance detection

**Step 2: Run tests to verify they fail**
- Run: `cargo test -p onchain_gateway logic -- --nocapture`
- Expected: FAIL due to unimplemented functions.

**Step 3: Implement minimal logic**
- Add pure helper functions in `logic.rs`.

**Step 4: Re-run tests**
- Run same command.
- Expected: PASS.

### Task 2: Program account model and instructions

**Files:**
- Create: `programs/onchain_gateway/src/error.rs`
- Create: `programs/onchain_gateway/src/instruction.rs`
- Create: `programs/onchain_gateway/src/state.rs`
- Create: `programs/onchain_gateway/src/processor.rs`
- Modify: `programs/onchain_gateway/src/lib.rs`

**Step 1: Write failing instruction tests**
- Add integration-style tests for instruction serialization and seed derivation.

**Step 2: Run tests to verify fail**
- Run: `cargo test -p onchain_gateway instruction -- --nocapture`

**Step 3: Implement minimal program code**
- Define account layouts and processor handlers:
  - initialize_gateway
  - register_consumer
  - topup
  - consume

**Step 4: Re-run tests**
- Ensure serialization + behavior tests pass.

### Task 3: CLI client (public testability)

**Files:**
- Create: `clients/gateway-cli/Cargo.toml`
- Create: `clients/gateway-cli/src/main.rs`

**Step 1: Write failing CLI parsing test**
- Add command parser tests for major commands.

**Step 2: Run tests to verify fail**
- Run: `cargo test -p gateway-cli -- --nocapture`

**Step 3: Implement minimal CLI**
- Support commands:
  - `init-gateway`
  - `register-consumer`
  - `topup`
  - `consume`
- Include `--rpc-url`, `--program-id`, `--keypair`.

**Step 4: Re-run tests**
- Ensure parser + instruction build tests pass.

### Task 4: README + challenge mapping

**Files:**
- Create: `README.md`

**Step 1: Document architecture and Web2 mapping**
- Describe Web2 equivalent backend flow.
- Describe Solana account/state-machine mapping.

**Step 2: Add tradeoffs/constraints and Devnet checklist**
- Include tx link placeholders and exact commands.

**Step 3: Add test + run instructions**
- Include local test commands and CLI examples.

### Task 5: Verification

**Files:**
- Modify: project files as needed

**Step 1: Format and lint-style checks**
- Run: `cargo fmt --all`

**Step 2: Run tests**
- Run: `cargo test --workspace`

**Step 3: Build CLI and program crates**
- Run: `cargo build --workspace`

**Step 4: Record known gaps**
- Note deployment/tx-link steps requiring real wallet and Devnet.
