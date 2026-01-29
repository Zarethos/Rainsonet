/**
 * Cryptographic functions for RAINSONET/RELYO SDK
 * Uses @noble/ed25519 for Ed25519 signatures
 * Uses @noble/hashes for BLAKE3 and SHA-256
 */

import * as ed from '@noble/ed25519';
import { blake3 } from '@noble/hashes/blake3';
import { sha256 } from '@noble/hashes/sha256';
import { bytesToHex, hexToBytes } from './utils';

/**
 * Generate a random Ed25519 keypair
 */
export async function generateKeypair(): Promise<{
  secretKey: Uint8Array;
  publicKey: Uint8Array;
}> {
  const secretKey = ed.utils.randomPrivateKey();
  const publicKey = await ed.getPublicKeyAsync(secretKey);
  
  return { secretKey, publicKey };
}

/**
 * Get public key from secret key
 */
export async function getPublicKey(secretKey: Uint8Array): Promise<Uint8Array> {
  return ed.getPublicKeyAsync(secretKey);
}

/**
 * Derive address from public key
 * Address is the BLAKE3 hash of the public key, truncated to 32 bytes
 */
export function deriveAddress(publicKey: Uint8Array): Uint8Array {
  return blake3(publicKey);
}

/**
 * Sign a message with Ed25519
 */
export async function sign(
  message: Uint8Array,
  secretKey: Uint8Array
): Promise<Uint8Array> {
  return ed.signAsync(message, secretKey);
}

/**
 * Verify an Ed25519 signature
 */
export async function verify(
  signature: Uint8Array,
  message: Uint8Array,
  publicKey: Uint8Array
): Promise<boolean> {
  try {
    return await ed.verifyAsync(signature, message, publicKey);
  } catch {
    return false;
  }
}

/**
 * BLAKE3 hash
 */
export function hashBlake3(data: Uint8Array): Uint8Array {
  return blake3(data);
}

/**
 * SHA-256 hash
 */
export function hashSha256(data: Uint8Array): Uint8Array {
  return sha256(data);
}

/**
 * Hash multiple values (concatenated)
 */
export function hashMultiple(...parts: Uint8Array[]): Uint8Array {
  const total = parts.reduce((sum, p) => sum + p.length, 0);
  const combined = new Uint8Array(total);
  let offset = 0;
  
  for (const part of parts) {
    combined.set(part, offset);
    offset += part.length;
  }
  
  return hashBlake3(combined);
}

/**
 * Create transaction hash (unique identifier)
 */
export function createTransactionHash(
  from: string,
  to: string,
  amount: string,
  fee: string,
  nonce: number,
  timestamp: number
): string {
  const encoder = new TextEncoder();
  const data = encoder.encode(
    `tx:${from}:${to}:${amount}:${fee}:${nonce}:${timestamp}`
  );
  
  return bytesToHex(hashBlake3(data));
}

/**
 * Create message to sign for transaction
 */
export function createSigningMessage(
  from: string,
  to: string,
  amount: string,
  fee: string,
  nonce: number,
  timestamp: number
): Uint8Array {
  const encoder = new TextEncoder();
  return encoder.encode(
    `RELYO:transfer:${from}:${to}:${amount}:${fee}:${nonce}:${timestamp}`
  );
}
