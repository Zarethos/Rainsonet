# RAINSONET

A decentralized payment infrastructure built for speed and simplicity.

## What is RAINSONET?

RAINSONET is a state-based payment system. Unlike traditional blockchains that store a chain of blocks, RAINSONET maintains account states directly. This means faster transactions and instant finality.

RELYO is the payment protocol that runs on top of RAINSONET. Think of RAINSONET as the engine and RELYO as the application layer for moving money around.

## Why build this?

Most blockchain systems are slow. Bitcoin takes an hour to confirm, Ethereum takes minutes. We wanted something that confirms in seconds without sacrificing decentralization.

The key differences:
- No mining or proof of work
- Validators vote on state changes directly
- Transactions finalize immediately once consensus is reached
- Simple account model (address to balance mapping)

## Project Structure

```
rainsonet/
├── core/           Core types and interfaces
├── crypto/         Ed25519 signatures, BLAKE3 hashing
├── state/          State storage (memory and disk)
├── p2p/            Peer-to-peer networking with libp2p
├── consensus/      Validator voting mechanism
├── modules/relyo/  Payment module
├── node/           Full node implementation
├── cli/            Command line tools
└── sdk/            TypeScript SDK for applications
```

## Building

You need Rust 1.75 or later.

```bash
cd rainsonet
cargo build --release
```

The binaries will be in `target/release/`.

## Running a Node

Start a local validator node:

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

## Using the CLI

Create a wallet:

```bash
./target/release/relyo wallet create mywallet
```

Check balance:

```bash
./target/release/relyo balance --wallet mywallet
```

Send tokens:

```bash
./target/release/relyo send --from mywallet --to <address> --amount 10
```

Check node status:

```bash
./target/release/relyo status
```

## TypeScript SDK

For JavaScript/TypeScript applications:

```bash
cd sdk
npm install
npm run build
```

Basic usage:

```typescript
import { Wallet, RelyoClient } from '@rainsonet/sdk';

const wallet = await Wallet.create();
const client = RelyoClient.devnet();

// check balance
const balance = await client.getBalance(wallet.address);

// send tokens
const tx = await client.send({
  wallet,
  to: recipientAddress,
  amount: 10
});
```

## API Endpoints

The node exposes a REST API:

| Endpoint | Method | Description |
|----------|--------|-------------|
| /health | GET | Health check |
| /status | GET | Node status |
| /account/:address | GET | Account info |
| /balance/:address | GET | Balance |
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
- Validator-based voting
- 2/3 majority required for finality
- No block reorganizations possible

## Token Economics

- Token: RELYO
- Decimals: 18
- Max supply: 100,000,000 RELYO
- Transaction fees are configurable
- Optional fee burn mechanism

## Current Limitations

This is version 1. Some things are not implemented yet:

- Multi-node networking (works locally)
- Validator staking
- State pruning
- Light clients

These are planned for future versions.

## Documentation

- [Architecture](docs/ARCHITECTURE.md) - System design and internals
- [SDK Guide](docs/SDK.md) - TypeScript SDK documentation

## License

Apache License 2.0. See [LICENSE](LICENSE) for details.
