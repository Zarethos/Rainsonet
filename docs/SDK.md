# RELYO SDK Documentation

## Installation

```bash
npm install @rainsonet/sdk
# or
yarn add @rainsonet/sdk
# or
pnpm add @rainsonet/sdk
```

## Quick Start

```typescript
import { Wallet, RelyoClient, Amount } from '@rainsonet/sdk';

// 1. Create a wallet
const wallet = await Wallet.create();
console.log('My address:', wallet.address);

// 2. Connect to a node
const client = RelyoClient.devnet('http://localhost:8080');

// 3. Check balance
const balance = await client.getBalance(wallet.address);
console.log('Balance:', balance.balanceRelyo, 'RELYO');

// 4. Send tokens
const response = await client.send({
  wallet,
  to: '0x...',
  amount: 10, // 10 RELYO
});

console.log('Transaction ID:', response.txId);
```

## Wallet Management

### Creating Wallets

```typescript
import { Wallet } from '@rainsonet/sdk';

// Random wallet
const wallet = await Wallet.create();

// From secret key
const imported = await Wallet.fromSecretKey('abc123...');

// From JSON
const restored = await Wallet.fromJSON('{"secretKey": "..."}');
```

### Wallet Properties

```typescript
// Get address (32 bytes hex)
console.log(wallet.address);

// Get public key (32 bytes hex)
console.log(wallet.publicKey);

// Export secret key (KEEP SAFE!)
console.log(wallet.exportSecretKey());

// Export to JSON
const json = wallet.toJSON();
```

### HD Wallets

```typescript
import { HDWallet } from '@rainsonet/sdk';

// Create from seed
const hdWallet = HDWallet.fromSeed(seedBytes);

// Derive wallets
const wallet0 = await hdWallet.deriveWallet(0);
const wallet1 = await hdWallet.deriveWallet(1);

// Derive multiple
const wallets = await hdWallet.deriveWallets(10);
```

## Transactions

### Creating Transactions

```typescript
import { Wallet, Amount } from '@rainsonet/sdk';

const wallet = await Wallet.create();

// Create and sign transaction
const tx = await wallet.createTransaction({
  to: recipientAddress,
  amount: Amount.toWei(10),  // 10 RELYO in wei
  fee: Amount.toWei(0.001),  // 0.001 RELYO fee
  nonce: 0,
});
```

### Using Transaction Builder

```typescript
import { TransactionBuilder, Amount } from '@rainsonet/sdk';

const params = new TransactionBuilder()
  .setRecipient('0x...')
  .setAmount(10)       // 10 RELYO
  .setFee(0.001)       // 0.001 RELYO
  .setNonce(0)
  .build();

const tx = await wallet.createTransaction(params);
```

### Validating Transactions

```typescript
import { validateTransaction, getTransactionId } from '@rainsonet/sdk';

// Validate
const result = await validateTransaction(tx);
if (!result.valid) {
  console.error('Invalid:', result.error);
}

// Get transaction ID
const txId = getTransactionId(tx);
```

## Client API

### Connecting

```typescript
import { RelyoClient, Networks } from '@rainsonet/sdk';

// Devnet (local)
const client = RelyoClient.devnet();

// Testnet
const client = RelyoClient.testnet();

// Mainnet
const client = RelyoClient.mainnet();

// Custom
const client = new RelyoClient({
  nodeUrl: 'http://my-node:8080',
  timeout: 30000,
  retries: 3,
});
```

### Node Status

```typescript
const status = await client.getStatus();
console.log('Node ID:', status.nodeId);
console.log('State Version:', status.stateVersion);
console.log('Peer Count:', status.peerCount);
console.log('Is Validator:', status.isValidator);
console.log('Mempool Size:', status.mempoolSize);
```

### Account Operations

```typescript
// Get full account info
const account = await client.getAccount(address);
console.log('Balance:', account.balance);
console.log('Nonce:', account.nonce);

// Get balance only
const balance = await client.getBalance(address);
console.log('Balance:', balance.balanceRelyo, 'RELYO');

// Get nonce only
const nonce = await client.getNonce(address);
```

### Sending Transactions

```typescript
// Low-level: manual nonce
const tx = await wallet.createTransaction({
  to: recipient,
  amount: Amount.toWei(10),
  fee: Amount.toWei(0.001),
  nonce: 0,
});
const response = await client.submitTransaction(tx);

// High-level: auto nonce
const response = await client.send({
  wallet,
  to: recipient,
  amount: 10,      // RELYO
  fee: 0.001,      // RELYO (optional)
});

console.log('TX ID:', response.txId);
console.log('Status:', response.status);
```

### Transaction Status

```typescript
// Get current status
const status = await client.getTransaction(txId);

// Wait for confirmation
const confirmed = await client.waitForTransaction(txId, {
  timeout: 60000,      // 60 seconds
  pollInterval: 1000,  // 1 second
});

if (confirmed.status === 'confirmed') {
  console.log('Transaction confirmed!');
}
```

## Amount Handling

```typescript
import { Amount } from '@rainsonet/sdk';

// Constants
Amount.ONE_RELYO   // BigInt: 1e18
Amount.DECIMALS    // 18

// Convert RELYO to wei
Amount.toWei(10)           // "10000000000000000000"
Amount.toWei(0.5)          // "500000000000000000"

// Convert wei to RELYO
Amount.fromWei("10000000000000000000")  // 10
Amount.fromWei(BigInt(10e18))           // 10

// Format for display
Amount.format("10000000000000000000")       // "10.0000"
Amount.format("10000000000000000000", 2)    // "10.00"
```

## Utilities

### Hex Conversion

```typescript
import { bytesToHex, hexToBytes, isValidHex } from '@rainsonet/sdk';

const hex = bytesToHex(new Uint8Array([1, 2, 3]));
const bytes = hexToBytes('010203');
const valid = isValidHex('abc123');
```

### Address Validation

```typescript
import { isValidAddress, truncateAddress } from '@rainsonet/sdk';

if (isValidAddress(address)) {
  console.log('Valid!');
}

// Display: "abc12345...6789abcd"
const short = truncateAddress(address, 8);
```

### Signing Messages

```typescript
// Sign arbitrary message
const signature = await wallet.signMessage('Hello, RAINSONET!');

// Verify (manual)
import { verify, hashBlake3 } from '@rainsonet/sdk';

const message = new TextEncoder().encode('Hello, RAINSONET!');
const isValid = await verify(
  hexToBytes(signature),
  message,
  hexToBytes(wallet.publicKey)
);
```

## Error Handling

```typescript
try {
  const response = await client.send({
    wallet,
    to: recipient,
    amount: 1000000, // More than balance
  });
} catch (error) {
  if (error.message.includes('insufficient balance')) {
    console.error('Not enough funds!');
  } else {
    console.error('Transaction failed:', error.message);
  }
}
```

## TypeScript Types

```typescript
import type {
  Address,
  Hash,
  Signature,
  PublicKey,
  AmountWei,
  AmountRelyo,
  Nonce,
  Timestamp,
  Account,
  BalanceInfo,
  SignedTransaction,
  TransactionResponse,
  NodeStatus,
  NetworkConfig,
} from '@rainsonet/sdk';
```

## Examples

### Payment Service

```typescript
import { Wallet, RelyoClient, Amount } from '@rainsonet/sdk';

class PaymentService {
  private wallet: Wallet;
  private client: RelyoClient;

  constructor(secretKey: string, nodeUrl: string) {
    this.client = new RelyoClient({ nodeUrl });
  }

  async init(secretKey: string) {
    this.wallet = await Wallet.fromSecretKey(secretKey);
  }

  async sendPayment(to: string, amount: number) {
    const response = await this.client.send({
      wallet: this.wallet,
      to,
      amount,
    });

    // Wait for confirmation
    const confirmed = await this.client.waitForTransaction(
      response.txId
    );

    return confirmed;
  }

  async getBalance() {
    const info = await this.client.getBalance(this.wallet.address);
    return Amount.fromWei(info.balance);
  }
}
```

### Batch Transactions

```typescript
async function sendBatch(
  wallet: Wallet,
  client: RelyoClient,
  recipients: { address: string; amount: number }[]
) {
  let nonce = await client.getNonce(wallet.address);
  const results = [];

  for (const { address, amount } of recipients) {
    const tx = await wallet.createTransaction({
      to: address,
      amount: Amount.toWei(amount),
      fee: Amount.toWei(0.001),
      nonce: nonce++,
    });

    const response = await client.submitTransaction(tx);
    results.push(response);
  }

  return results;
}
```

## Networks

| Network | Chain ID | RPC URL |
|---------|----------|---------|
| Mainnet | 1 | https://mainnet.rainsonet.io |
| Testnet | 2 | https://testnet.rainsonet.io |
| Devnet | 3 | http://127.0.0.1:8080 |

## Best Practices

1. **Never expose secret keys** - Use environment variables
2. **Always validate addresses** - Before sending
3. **Handle errors gracefully** - Network failures happen
4. **Use appropriate fees** - Check network conditions
5. **Wait for confirmations** - For critical transactions
6. **Store wallets securely** - Encrypt at rest
