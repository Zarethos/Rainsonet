/**
 * Transaction utilities for RAINSONET/RELYO SDK
 */

import { verify, createTransactionHash, createSigningMessage } from './crypto';
import { hexToBytes, isValidAddress, isValidAmount, bytesToHex } from './utils';
import type { SignedTransaction, Hash, Address, AmountWei, Nonce } from './types';
import { Amount } from './types';

/**
 * Validate a signed transaction
 */
export async function validateTransaction(tx: SignedTransaction): Promise<{
  valid: boolean;
  error?: string;
}> {
  // Validate addresses
  if (!isValidAddress(tx.from)) {
    return { valid: false, error: 'Invalid sender address' };
  }
  
  if (!isValidAddress(tx.to)) {
    return { valid: false, error: 'Invalid recipient address' };
  }
  
  // Validate amounts
  if (!isValidAmount(tx.amount)) {
    return { valid: false, error: 'Invalid amount' };
  }
  
  if (!isValidAmount(tx.fee)) {
    return { valid: false, error: 'Invalid fee' };
  }
  
  // Cannot send to self
  if (tx.from === tx.to) {
    return { valid: false, error: 'Cannot send to self' };
  }
  
  // Validate signature
  const message = createSigningMessage(
    tx.from,
    tx.to,
    tx.amount,
    tx.fee,
    tx.nonce,
    tx.timestamp
  );
  
  const signature = hexToBytes(tx.signature);
  const publicKey = hexToBytes(tx.publicKey);
  
  const isValid = await verify(signature, message, publicKey);
  
  if (!isValid) {
    return { valid: false, error: 'Invalid signature' };
  }
  
  return { valid: true };
}

/**
 * Calculate transaction ID
 */
export function getTransactionId(tx: SignedTransaction): Hash {
  return createTransactionHash(
    tx.from,
    tx.to,
    tx.amount,
    tx.fee,
    tx.nonce,
    tx.timestamp
  );
}

/**
 * Calculate total cost of transaction (amount + fee)
 */
export function calculateTotalCost(tx: SignedTransaction): bigint {
  return BigInt(tx.amount) + BigInt(tx.fee);
}

/**
 * Transaction builder for easier transaction creation
 */
export class TransactionBuilder {
  private to?: Address;
  private amount?: AmountWei;
  private fee?: AmountWei;
  private nonce?: Nonce;
  private timestamp?: number;
  
  /**
   * Set recipient
   */
  setRecipient(to: Address): this {
    if (!isValidAddress(to)) {
      throw new Error('Invalid recipient address');
    }
    this.to = to;
    return this;
  }
  
  /**
   * Set amount in RELYO
   */
  setAmount(relyo: number): this {
    this.amount = Amount.toWei(relyo);
    return this;
  }
  
  /**
   * Set amount in wei
   */
  setAmountWei(wei: string | bigint): this {
    this.amount = wei.toString();
    return this;
  }
  
  /**
   * Set fee in RELYO
   */
  setFee(relyo: number): this {
    this.fee = Amount.toWei(relyo);
    return this;
  }
  
  /**
   * Set fee in wei
   */
  setFeeWei(wei: string | bigint): this {
    this.fee = wei.toString();
    return this;
  }
  
  /**
   * Set nonce
   */
  setNonce(nonce: Nonce): this {
    this.nonce = nonce;
    return this;
  }
  
  /**
   * Set timestamp
   */
  setTimestamp(timestamp: number): this {
    this.timestamp = timestamp;
    return this;
  }
  
  /**
   * Build the transaction parameters
   */
  build(): {
    to: Address;
    amount: AmountWei;
    fee: AmountWei;
    nonce: Nonce;
    timestamp?: number;
  } {
    if (!this.to) {
      throw new Error('Recipient not set');
    }
    
    if (!this.amount) {
      throw new Error('Amount not set');
    }
    
    if (!this.fee) {
      throw new Error('Fee not set');
    }
    
    if (this.nonce === undefined) {
      throw new Error('Nonce not set');
    }
    
    return {
      to: this.to,
      amount: this.amount,
      fee: this.fee,
      nonce: this.nonce,
      timestamp: this.timestamp,
    };
  }
}
