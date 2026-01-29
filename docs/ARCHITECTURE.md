# RAINSONET Architecture

## Overview

RAINSONET is a state-based decentralized payment system, fundamentally different from traditional blockchain architectures. Instead of maintaining a chain of blocks, RAINSONET maintains a **state tree** representing account balances, updated atomically through consensus.

## Design Philosophy

### Why Not Blockchain?

| Blockchain | RAINSONET |
|------------|-----------|
| Chain of blocks | State tree |
| Mining/PoW/PoS | Validator voting |
| Probabilistic finality | Deterministic finality |
| Block time delays | Instant confirmation |
| Complex fee markets | Simple fee model |
| Reorganizations possible | No reorganizations |

### Core Principles

1. **Simplicity** - Minimal complexity, focused scope
2. **Security** - Audited cryptography, no custom algorithms
3. **Performance** - Sub-second finality, high throughput
4. **Modularity** - Pluggable components, easy to extend

## System Components

### Layer 1: Core Types

```
┌─────────────────────────────────────┐
│             Core Types              │
├─────────────────────────────────────┤
│ Address    │ 32-byte account ID     │
│ Hash       │ 32-byte BLAKE3 hash    │
│ Signature  │ 64-byte Ed25519 sig    │
│ PublicKey  │ 32-byte Ed25519 pk     │
│ Amount     │ u128 (18 decimals)     │
│ Nonce      │ u64 sequence number    │
│ Timestamp  │ u64 milliseconds       │
└─────────────────────────────────────┘
```

### Layer 2: Cryptography

```
┌─────────────────────────────────────┐
│           Cryptography              │
├─────────────────────────────────────┤
│ Key Generation                      │
│ └─ Ed25519 keypair                  │
│                                     │
│ Signing                             │
│ └─ Ed25519 sign/verify              │
│                                     │
│ Hashing                             │
│ └─ BLAKE3 (primary)                 │
│ └─ SHA-256 (fallback)               │
│                                     │
│ Derivation                          │
│ └─ HKDF for child keys              │
└─────────────────────────────────────┘
```

### Layer 3: State Management

```
┌─────────────────────────────────────┐
│         State Management            │
├─────────────────────────────────────┤
│ State Store Interface               │
│ ├─ get(key) → value                 │
│ ├─ set(key, value)                  │
│ ├─ delete(key)                      │
│ └─ compute_root() → StateRoot       │
│                                     │
│ Implementations                     │
│ ├─ MemoryStateStore (testing)       │
│ └─ PersistentStateStore (sled)      │
│                                     │
│ Snapshots                           │
│ └─ Point-in-time state backup       │
└─────────────────────────────────────┘
```

### Layer 4: Networking

```
┌─────────────────────────────────────┐
│           P2P Network               │
├─────────────────────────────────────┤
│ libp2p Stack                        │
│ ├─ Transport: TCP + Noise           │
│ ├─ Multiplexing: Yamux              │
│ ├─ Discovery: mDNS (local)          │
│ └─ Messaging: Gossipsub             │
│                                     │
│ Message Types                       │
│ ├─ Transaction                      │
│ ├─ Proposal                         │
│ ├─ Vote                             │
│ ├─ SyncRequest/Response             │
│ └─ Ping/Pong                        │
└─────────────────────────────────────┘
```

### Layer 5: Consensus

```
┌─────────────────────────────────────┐
│            Consensus                │
├─────────────────────────────────────┤
│ Validator Set                       │
│ └─ Dynamic membership               │
│                                     │
│ Proposal                            │
│ ├─ Previous state root              │
│ ├─ New state root                   │
│ ├─ Transaction list                 │
│ └─ State changes                    │
│                                     │
│ Voting                              │
│ ├─ Accept / Reject                  │
│ └─ 2/3 majority required            │
│                                     │
│ Finality                            │
│ └─ Deterministic, no reorg          │
└─────────────────────────────────────┘
```

### Layer 6: RELYO Module

```
┌─────────────────────────────────────┐
│          RELYO Payment              │
├─────────────────────────────────────┤
│ Account Model                       │
│ └─ address → {balance, nonce}       │
│                                     │
│ Transaction                         │
│ ├─ from, to, amount, fee            │
│ ├─ nonce (replay protection)        │
│ └─ signature                        │
│                                     │
│ Validation Rules                    │
│ ├─ Valid signature                  │
│ ├─ Sufficient balance               │
│ ├─ Correct nonce                    │
│ └─ Amount > 0                       │
│                                     │
│ Mempool                             │
│ └─ Priority queue by fee            │
└─────────────────────────────────────┘
```

## Transaction Flow

```
User                Node               Consensus         State
  │                   │                    │               │
  │ Create TX         │                    │               │
  │──────────────────>│                    │               │
  │                   │                    │               │
  │                   │ Validate           │               │
  │                   │ (sig, balance)     │               │
  │                   │                    │               │
  │                   │ Add to Mempool     │               │
  │                   │                    │               │
  │                   │ Create Proposal    │               │
  │                   │───────────────────>│               │
  │                   │                    │               │
  │                   │                    │ Validators    │
  │                   │                    │ Vote          │
  │                   │                    │               │
  │                   │                    │ 2/3 Accept    │
  │                   │                    │               │
  │                   │ Finalized          │               │
  │                   │<───────────────────│               │
  │                   │                    │               │
  │                   │ Apply Changes      │               │
  │                   │───────────────────────────────────>│
  │                   │                    │               │
  │ TX Confirmed      │                    │               │
  │<──────────────────│                    │               │
```

## State Model

### Account State

```rust
struct AccountState {
    balance: Amount,    // Current balance in wei
    nonce: Nonce,       // Transaction count
}
```

### State Tree

```
                    State Root
                         │
        ┌────────────────┼────────────────┐
        │                │                │
    Account A        Account B        Account C
   ┌─────────┐      ┌─────────┐      ┌─────────┐
   │bal: 100 │      │bal: 50  │      │bal: 200 │
   │nonce: 5 │      │nonce: 2 │      │nonce: 0 │
   └─────────┘      └─────────┘      └─────────┘
```

### State Transitions

```
State S₀ + Transaction T → State S₁

Where:
- S₀.accounts[from].balance -= (amount + fee)
- S₀.accounts[from].nonce += 1
- S₀.accounts[to].balance += amount
- fee is optionally burned
```

## Consensus Protocol

### Validator Selection

- Fixed validator set (v1)
- Dynamic membership planned (v2)
- Stake-weighted voting planned

### Proposal-Vote Flow

1. **Propose**: Leader creates proposal with transactions
2. **Vote**: Validators verify and vote Accept/Reject
3. **Finalize**: 2/3+ Accept → Apply to state
4. **Reject**: 2/3+ Reject → Discard proposal

### Finality Guarantee

Once finalized:
- State change is permanent
- No reorganizations possible
- All nodes converge to same state

## Security Model

### Threat Model

| Threat | Mitigation |
|--------|------------|
| Double spend | Nonce + mempool |
| Replay attack | Per-account nonce |
| Signature forgery | Ed25519 security |
| State corruption | Merkle proofs |
| Network partition | Consensus voting |
| Validator collusion | 2/3 requirement |

### Cryptographic Guarantees

1. **Authenticity**: Ed25519 signatures
2. **Integrity**: BLAKE3 hashing
3. **Confidentiality**: Noise protocol (network)
4. **Non-repudiation**: Signed transactions

## Performance Characteristics

| Metric | Target |
|--------|--------|
| Transaction throughput | 1000+ TPS |
| Finality time | < 3 seconds |
| State read latency | < 1 ms |
| Network propagation | < 500 ms |

## Scalability Considerations

### Current Limitations

- Single state tree
- All nodes store full state
- Sequential transaction processing

### Future Improvements

- State sharding
- Parallel validation
- Light client support
- Pruning old state

## Comparison with Other Systems

| Feature | RAINSONET | Ethereum | Bitcoin |
|---------|-----------|----------|---------|
| Model | State | Account | UTXO |
| Consensus | Voting | PoS | PoW |
| Finality | Instant | ~15 min | ~60 min |
| TPS | 1000+ | ~30 | ~7 |
| Smart Contracts | No | Yes | Limited |
| Complexity | Low | High | Medium |
