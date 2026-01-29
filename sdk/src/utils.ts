/**
 * Utility functions for RAINSONET/RELYO SDK
 */

/**
 * Convert bytes to hex string
 */
export function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');
}

/**
 * Convert hex string to bytes
 */
export function hexToBytes(hex: string): Uint8Array {
  const cleanHex = hex.startsWith('0x') ? hex.slice(2) : hex;
  
  if (cleanHex.length % 2 !== 0) {
    throw new Error('Invalid hex string length');
  }
  
  const bytes = new Uint8Array(cleanHex.length / 2);
  
  for (let i = 0; i < bytes.length; i++) {
    bytes[i] = parseInt(cleanHex.substr(i * 2, 2), 16);
  }
  
  return bytes;
}

/**
 * Check if string is valid hex
 */
export function isValidHex(value: string): boolean {
  const hex = value.startsWith('0x') ? value.slice(2) : value;
  return /^[0-9a-fA-F]*$/.test(hex) && hex.length % 2 === 0;
}

/**
 * Check if string is valid address (64 hex chars)
 */
export function isValidAddress(address: string): boolean {
  const hex = address.startsWith('0x') ? address.slice(2) : address;
  return isValidHex(address) && hex.length === 64;
}

/**
 * Validate amount is positive
 */
export function isValidAmount(amount: string | bigint): boolean {
  try {
    const value = typeof amount === 'string' ? BigInt(amount) : amount;
    return value >= 0n;
  } catch {
    return false;
  }
}

/**
 * Truncate address for display
 */
export function truncateAddress(address: string, chars: number = 8): string {
  const hex = address.startsWith('0x') ? address.slice(2) : address;
  
  if (hex.length <= chars * 2) {
    return address;
  }
  
  return `${hex.slice(0, chars)}...${hex.slice(-chars)}`;
}

/**
 * Format amount with units
 */
export function formatAmount(
  wei: string | bigint,
  decimals: number = 18,
  precision: number = 4
): string {
  const value = typeof wei === 'string' ? BigInt(wei) : wei;
  const divisor = BigInt(10 ** decimals);
  
  const whole = value / divisor;
  const remainder = value % divisor;
  
  const remainderStr = remainder.toString().padStart(decimals, '0');
  const truncatedRemainder = remainderStr.slice(0, precision);
  
  return `${whole}.${truncatedRemainder}`;
}

/**
 * Sleep for specified milliseconds
 */
export function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Retry function with exponential backoff
 */
export async function retry<T>(
  fn: () => Promise<T>,
  maxAttempts: number = 3,
  baseDelay: number = 1000
): Promise<T> {
  let lastError: Error | undefined;
  
  for (let attempt = 1; attempt <= maxAttempts; attempt++) {
    try {
      return await fn();
    } catch (err) {
      lastError = err instanceof Error ? err : new Error(String(err));
      
      if (attempt < maxAttempts) {
        const delay = baseDelay * Math.pow(2, attempt - 1);
        await sleep(delay);
      }
    }
  }
  
  throw lastError;
}

/**
 * Create a unique ID
 */
export function createUniqueId(): string {
  const timestamp = Date.now().toString(36);
  const random = Math.random().toString(36).substring(2, 10);
  return `${timestamp}-${random}`;
}

/**
 * Deep clone an object
 */
export function deepClone<T>(obj: T): T {
  return JSON.parse(JSON.stringify(obj));
}

/**
 * Encode data to base64
 */
export function toBase64(data: Uint8Array): string {
  if (typeof Buffer !== 'undefined') {
    return Buffer.from(data).toString('base64');
  }
  // Browser fallback
  return btoa(String.fromCharCode(...data));
}

/**
 * Decode base64 to bytes
 */
export function fromBase64(base64: string): Uint8Array {
  if (typeof Buffer !== 'undefined') {
    return new Uint8Array(Buffer.from(base64, 'base64'));
  }
  // Browser fallback
  return new Uint8Array(
    atob(base64)
      .split('')
      .map((c) => c.charCodeAt(0))
  );
}
