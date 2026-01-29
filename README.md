# RAINSONET

Open-source software for building decentralized state replication networks.

## Overview

RAINSONET is a consensus engine that replicates state across a distributed network of nodes. It provides the primitives for building applications that require agreement on shared state without a central coordinator.

This repository contains:
- The core consensus engine (RAINSONET)
- A reference implementation of a value transfer module (RELYO)
- Development tools (CLI, SDK)

RELYO is one possible module built on the engine. The engine itself is general-purpose and can be used for other applications requiring distributed state agreement.

## Architecture

RAINSONET uses a state-based model rather than a chain-of-blocks model. Nodes maintain a mapping of addresses to values, and validators reach agreement on state transitions through a voting protocol.

Key design choices:
- Validators vote on proposed state changes
- State transitions require 2/3 validator agreement
- Once finalized, state changes are not reversible
- Account-based model (address to value mapping)

```
rainsonet/
├── core/           Core types and interfaces
├── crypto/         Ed25519 signatures, BLAKE3 hashing
├── state/          State storage (memory and disk)
├── p2p/            Peer-to-peer networking with libp2p
├── consensus/      Validator voting mechanism
├── modules/relyo/  Reference value transfer module
├── node/           Node implementation
├── cli/            Command line tools
└── sdk/            TypeScript SDK
```

## Building

Requires Rust 1.75 or later.

```bash
cd rainsonet
cargo build --release
```

Binaries output to `target/release/`.

## Running a Node

Start a local node:

```bash
./target/release/rainsonet-node run --validator --api-addr 127.0.0.1:8080
```

Generate a keypair:

```bash
./target/release/rainsonet-node keygen
```

Create genesis configuration:

```bash
./target/release/rainsonet-node genesis --output genesis.json
```

## CLI Usage

Create a keypair:

```bash
./target/release/relyo wallet create mywallet
```

Query state:

```bash
./target/release/relyo balance --wallet mywallet
```

Submit a transaction:

```bash
./target/release/relyo send --from mywallet --to <address> --amount 10
```

Query node:

```bash
./target/release/relyo status
```

## SDK

TypeScript SDK for building applications:

```bash
cd sdk
npm install
npm run build
```

Example:

```typescript
import { Wallet, RelyoClient } from '@rainsonet/sdk';

const wallet = await Wallet.create();
const client = RelyoClient.devnet();

const balance = await client.getBalance(wallet.address);

const tx = await client.send({
  wallet,
  to: recipientAddress,
  amount: 10
});
```

## API

The node exposes an HTTP API:

| Endpoint | Method | Description |
|----------|--------|-------------|
| /health | GET | Health check |
| /status | GET | Node status |
| /account/:address | GET | Account state |
| /balance/:address | GET | Account balance |
| /transaction | POST | Submit transaction |
| /transaction/:id | GET | Transaction status |

## Technical Details

Cryptography:
- Ed25519 for signatures (ed25519-dalek)
- BLAKE3 for hashing
- HKDF for key derivation

Networking:
- libp2p for peer-to-peer communication
- Gossipsub for message propagation
- mDNS for local peer discovery

Storage:
- In-memory store for development
- sled embedded database for persistence

Consensus:
- Validator-based voting protocol
- 2/3 majority required for state finalization
- Deterministic finality (no reorganizations)

## RELYO Module

The RELYO module is a reference implementation demonstrating value transfer on the RAINSONET engine. It implements:

- Account balances (address to uint mapping)
- Signed transactions (from, to, amount, fee, nonce)
- Nonce-based replay protection
- Configurable transaction fees

Configuration parameters in the reference implementation:
- Unit decimals: 18
- Genesis allocations: configurable
- Fee handling: configurable

The RELYO module is provided as example code. It is not a financial product or service.

## Current Status

This is experimental software under active development.

Not yet implemented:
- Multi-node network bootstrapping
- Validator rotation
- State pruning
- Light client protocol

## Documentation

- [Architecture](docs/ARCHITECTURE.md) - Design documentation
- [SDK Guide](docs/SDK.md) - SDK documentation

## Disclaimer

This software is provided as-is under the Apache License 2.0.

RAINSONET is open-source software for research and development purposes. It is not a financial service, payment processor, money transmitter, or custodian. The developers do not operate any network, hold any funds, or provide any services.

Users who run this software do so at their own risk and are responsible for compliance with applicable laws in their jurisdiction.

The RELYO module is reference code demonstrating the engine's capabilities. Any units tracked by the module are internal accounting units with no inherent value.

## License

Apache License 2.0. See [LICENSE](LICENSE) for details.
