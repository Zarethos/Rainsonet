/**
 * Core types for RAINSONET/RELYO SDK
 */

/** 32-byte address (hex string) */
export type Address = string;

/** 32-byte hash (hex string) */
export type Hash = string;

/** 64-byte signature (hex string) */
export type Signature = string;

/** 32-byte public key (hex string) */
export type PublicKey = string;

/** Amount in wei (string to handle large numbers) */
export type AmountWei = string;

/** Amount in RELYO (number for convenience) */
export type AmountRelyo = number;

/** Transaction nonce */
export type Nonce = number;

/** Unix timestamp in milliseconds */
export type Timestamp = number;

/**
 * Account state
 */
export interface Account {
  address: Address;
  balance: AmountWei;
  nonce: Nonce;
}

/**
 * Balance info
 */
export interface BalanceInfo {
  address: Address;
  balance: AmountWei;
  balanceRelyo: string;
}

/**
 * Transaction data
 */
export interface TransactionData {
  from: Address;
  to: Address;
  amount: AmountWei;
  fee: AmountWei;
  nonce: Nonce;
  timestamp: Timestamp;
}

/**
 * Signed transaction
 */
export interface SignedTransaction extends TransactionData {
  publicKey: PublicKey;
  signature: Signature;
}

/**
 * Transaction response from node
 */
export interface TransactionResponse {
  txId: Hash;
  status: 'pending' | 'confirmed' | 'failed' | 'unknown';
}

/**
 * Node status
 */
export interface NodeStatus {
  nodeId: string;
  stateVersion: number;
  stateRoot: Hash;
  peerCount: number;
  isValidator: boolean;
  mempoolSize: number;
}

/**
 * API response wrapper
 */
export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
}

/**
 * Network configuration
 */
export interface NetworkConfig {
  chainId: number;
  chainName: string;
  nodeUrl: string;
}

/**
 * Predefined networks
 */
export const Networks = {
  mainnet: {
    chainId: 1,
    chainName: 'RAINSONET Mainnet',
    nodeUrl: 'https://mainnet.rainsonet.io',
  } as NetworkConfig,
  
  testnet: {
    chainId: 2,
    chainName: 'RAINSONET Testnet',
    nodeUrl: 'https://testnet.rainsonet.io',
  } as NetworkConfig,
  
  devnet: {
    chainId: 3,
    chainName: 'RAINSONET Devnet',
    nodeUrl: 'http://127.0.0.1:8080',
  } as NetworkConfig,
} as const;

/**
 * Amount constants
 */
export const Amount = {
  /** One RELYO in wei (10^18) */
  ONE_RELYO: BigInt('1000000000000000000'),
  
  /** Decimals */
  DECIMALS: 18,
  
  /** Convert RELYO to wei */
  toWei(relyo: number | string): string {
    const num = typeof relyo === 'string' ? parseFloat(relyo) : relyo;
    const wei = BigInt(Math.floor(num * 1e18));
    return wei.toString();
  },
  
  /** Convert wei to RELYO */
  fromWei(wei: string | bigint): number {
    const weiBigInt = typeof wei === 'string' ? BigInt(wei) : wei;
    return Number(weiBigInt) / 1e18;
  },
  
  /** Format wei as RELYO string with decimals */
  format(wei: string | bigint, decimals: number = 4): string {
    const relyo = this.fromWei(wei);
    return relyo.toFixed(decimals);
  },
};
