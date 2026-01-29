/**
 * Wallet implementation for RAINSONET/RELYO SDK
 */

import {
  generateKeypair,
  getPublicKey,
  deriveAddress,
  sign,
  createSigningMessage,
} from './crypto';
import { bytesToHex, hexToBytes, isValidAddress } from './utils';
import type {
  Address,
  PublicKey,
  SignedTransaction,
  TransactionData,
  AmountWei,
  Nonce,
} from './types';
import { Amount } from './types';

/**
 * RELYO Wallet
 * 
 * Manages keypair and signing operations
 */
export class Wallet {
  private secretKey: Uint8Array;
  private _publicKey: Uint8Array;
  private _address: Uint8Array;
  
  private constructor(
    secretKey: Uint8Array,
    publicKey: Uint8Array,
    address: Uint8Array
  ) {
    this.secretKey = secretKey;
    this._publicKey = publicKey;
    this._address = address;
  }
  
  /**
   * Create a new random wallet
   */
  static async create(): Promise<Wallet> {
    const { secretKey, publicKey } = await generateKeypair();
    const address = deriveAddress(publicKey);
    
    return new Wallet(secretKey, publicKey, address);
  }
  
  /**
   * Import wallet from secret key
   */
  static async fromSecretKey(secretKeyHex: string): Promise<Wallet> {
    const secretKey = hexToBytes(secretKeyHex);
    
    if (secretKey.length !== 32) {
      throw new Error('Invalid secret key length');
    }
    
    const publicKey = await getPublicKey(secretKey);
    const address = deriveAddress(publicKey);
    
    return new Wallet(secretKey, publicKey, address);
  }
  
  /**
   * Import wallet from JSON
   */
  static async fromJSON(json: string): Promise<Wallet> {
    const data = JSON.parse(json);
    
    if (!data.secretKey) {
      throw new Error('Invalid wallet JSON: missing secretKey');
    }
    
    return Wallet.fromSecretKey(data.secretKey);
  }
  
  /**
   * Get address as hex string
   */
  get address(): Address {
    return bytesToHex(this._address);
  }
  
  /**
   * Get public key as hex string
   */
  get publicKey(): PublicKey {
    return bytesToHex(this._publicKey);
  }
  
  /**
   * Export secret key as hex string
   * 
   * ⚠️ CAUTION: Keep this secure!
   */
  exportSecretKey(): string {
    return bytesToHex(this.secretKey);
  }
  
  /**
   * Export wallet to JSON
   * 
   * ⚠️ CAUTION: Contains secret key!
   */
  toJSON(): string {
    return JSON.stringify({
      address: this.address,
      publicKey: this.publicKey,
      secretKey: this.exportSecretKey(),
    });
  }
  
  /**
   * Sign arbitrary data
   */
  async signData(data: Uint8Array): Promise<Uint8Array> {
    return sign(data, this.secretKey);
  }
  
  /**
   * Sign a message (string)
   */
  async signMessage(message: string): Promise<string> {
    const encoder = new TextEncoder();
    const data = encoder.encode(message);
    const signature = await this.signData(data);
    return bytesToHex(signature);
  }
  
  /**
   * Create and sign a transaction
   */
  async createTransaction(params: {
    to: Address;
    amount: AmountWei | number;
    fee: AmountWei | number;
    nonce: Nonce;
    timestamp?: number;
  }): Promise<SignedTransaction> {
    if (!isValidAddress(params.to)) {
      throw new Error('Invalid recipient address');
    }
    
    // Convert amounts to wei strings
    const amount = typeof params.amount === 'number'
      ? Amount.toWei(params.amount)
      : params.amount;
    
    const fee = typeof params.fee === 'number'
      ? Amount.toWei(params.fee)
      : params.fee;
    
    const timestamp = params.timestamp ?? Date.now();
    
    // Create signing message
    const message = createSigningMessage(
      this.address,
      params.to,
      amount,
      fee,
      params.nonce,
      timestamp
    );
    
    // Sign
    const signatureBytes = await sign(message, this.secretKey);
    const signature = bytesToHex(signatureBytes);
    
    return {
      from: this.address,
      to: params.to,
      amount,
      fee,
      nonce: params.nonce,
      timestamp,
      publicKey: this.publicKey,
      signature,
    };
  }
}

/**
 * HD Wallet (Hierarchical Deterministic)
 * 
 * Derive multiple wallets from a seed
 */
export class HDWallet {
  private masterSeed: Uint8Array;
  
  private constructor(seed: Uint8Array) {
    this.masterSeed = seed;
  }
  
  /**
   * Create from seed bytes
   */
  static fromSeed(seed: Uint8Array): HDWallet {
    if (seed.length < 32) {
      throw new Error('Seed must be at least 32 bytes');
    }
    return new HDWallet(seed);
  }
  
  /**
   * Create from hex seed
   */
  static fromHex(seedHex: string): HDWallet {
    return HDWallet.fromSeed(hexToBytes(seedHex));
  }
  
  /**
   * Derive wallet at index
   */
  async deriveWallet(index: number): Promise<Wallet> {
    const { blake3 } = await import('@noble/hashes/blake3');
    
    // Derive child key using HKDF-like derivation
    const indexBytes = new Uint8Array(4);
    new DataView(indexBytes.buffer).setUint32(0, index, false);
    
    const combined = new Uint8Array(this.masterSeed.length + 4);
    combined.set(this.masterSeed);
    combined.set(indexBytes, this.masterSeed.length);
    
    const childKey = blake3(combined);
    
    return Wallet.fromSecretKey(bytesToHex(childKey));
  }
  
  /**
   * Derive multiple wallets
   */
  async deriveWallets(count: number, startIndex: number = 0): Promise<Wallet[]> {
    const wallets: Wallet[] = [];
    
    for (let i = 0; i < count; i++) {
      wallets.push(await this.deriveWallet(startIndex + i));
    }
    
    return wallets;
  }
}
