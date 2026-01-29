/**
 * API Client for RAINSONET/RELYO nodes
 */

import type {
  Address,
  Hash,
  Account,
  BalanceInfo,
  SignedTransaction,
  TransactionResponse,
  NodeStatus,
  ApiResponse,
  NetworkConfig,
} from './types';
import { Networks } from './types';
import { retry } from './utils';
import { Wallet } from './wallet';
import { TransactionBuilder, getTransactionId } from './transaction';

/**
 * Client configuration
 */
export interface ClientConfig {
  network?: NetworkConfig;
  nodeUrl?: string;
  timeout?: number;
  retries?: number;
}

/**
 * RELYO API Client
 * 
 * Interact with RAINSONET nodes
 */
export class RelyoClient {
  private baseUrl: string;
  private timeout: number;
  private retries: number;
  private network: NetworkConfig;
  
  constructor(config: ClientConfig = {}) {
    this.network = config.network ?? Networks.devnet;
    this.baseUrl = (config.nodeUrl ?? this.network.nodeUrl).replace(/\/$/, '');
    this.timeout = config.timeout ?? 30000;
    this.retries = config.retries ?? 3;
  }
  
  /**
   * Create client for mainnet
   */
  static mainnet(): RelyoClient {
    return new RelyoClient({ network: Networks.mainnet });
  }
  
  /**
   * Create client for testnet
   */
  static testnet(): RelyoClient {
    return new RelyoClient({ network: Networks.testnet });
  }
  
  /**
   * Create client for local devnet
   */
  static devnet(nodeUrl?: string): RelyoClient {
    return new RelyoClient({
      network: Networks.devnet,
      nodeUrl,
    });
  }
  
  /**
   * Make API request
   */
  private async request<T>(
    method: string,
    path: string,
    body?: unknown
  ): Promise<T> {
    const url = `${this.baseUrl}${path}`;
    
    const fetchFn = async () => {
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), this.timeout);
      
      try {
        const response = await fetch(url, {
          method,
          headers: {
            'Content-Type': 'application/json',
          },
          body: body ? JSON.stringify(body) : undefined,
          signal: controller.signal,
        });
        
        clearTimeout(timeoutId);
        
        if (!response.ok) {
          throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }
        
        const json: ApiResponse<T> = await response.json();
        
        if (!json.success) {
          throw new Error(json.error ?? 'Unknown error');
        }
        
        if (json.data === undefined) {
          throw new Error('Empty response');
        }
        
        return json.data;
      } finally {
        clearTimeout(timeoutId);
      }
    };
    
    return retry(fetchFn, this.retries);
  }
  
  /**
   * Get node status
   */
  async getStatus(): Promise<NodeStatus> {
    const data = await this.request<{
      node_id: string;
      state_version: number;
      state_root: string;
      peer_count: number;
      is_validator: boolean;
      mempool_size: number;
    }>('GET', '/status');
    
    return {
      nodeId: data.node_id,
      stateVersion: data.state_version,
      stateRoot: data.state_root,
      peerCount: data.peer_count,
      isValidator: data.is_validator,
      mempoolSize: data.mempool_size,
    };
  }
  
  /**
   * Get account info
   */
  async getAccount(address: Address): Promise<Account> {
    const data = await this.request<{
      address: string;
      balance: string;
      nonce: number;
    }>('GET', `/account/${address}`);
    
    return {
      address: data.address,
      balance: data.balance,
      nonce: data.nonce,
    };
  }
  
  /**
   * Get balance
   */
  async getBalance(address: Address): Promise<BalanceInfo> {
    const data = await this.request<{
      address: string;
      balance: string;
      balance_relyo: string;
    }>('GET', `/balance/${address}`);
    
    return {
      address: data.address,
      balance: data.balance,
      balanceRelyo: data.balance_relyo,
    };
  }
  
  /**
   * Get account nonce
   */
  async getNonce(address: Address): Promise<number> {
    const account = await this.getAccount(address);
    return account.nonce;
  }
  
  /**
   * Submit transaction
   */
  async submitTransaction(tx: SignedTransaction): Promise<TransactionResponse> {
    const data = await this.request<{
      tx_id: string;
      status: string;
    }>('POST', '/transaction', {
      from: tx.from,
      to: tx.to,
      amount: tx.amount,
      fee: tx.fee,
      nonce: tx.nonce,
      public_key: tx.publicKey,
      signature: tx.signature,
    });
    
    return {
      txId: data.tx_id,
      status: data.status as TransactionResponse['status'],
    };
  }
  
  /**
   * Get transaction status
   */
  async getTransaction(txId: Hash): Promise<TransactionResponse> {
    const data = await this.request<{
      tx_id: string;
      status: string;
    }>('GET', `/transaction/${txId}`);
    
    return {
      txId: data.tx_id,
      status: data.status as TransactionResponse['status'],
    };
  }
  
  /**
   * Send RELYO tokens
   * 
   * Convenience method that handles nonce fetching
   */
  async send(params: {
    wallet: Wallet;
    to: Address;
    amount: number;
    fee?: number;
  }): Promise<TransactionResponse> {
    // Fetch current nonce
    const nonce = await this.getNonce(params.wallet.address);
    
    // Create transaction
    const tx = await params.wallet.createTransaction({
      to: params.to,
      amount: params.amount,
      fee: params.fee ?? 0.001,
      nonce,
    });
    
    // Submit
    return this.submitTransaction(tx);
  }
  
  /**
   * Wait for transaction to be confirmed
   */
  async waitForTransaction(
    txId: Hash,
    timeoutMs: number = 60000,
    pollIntervalMs: number = 1000
  ): Promise<TransactionResponse> {
    const startTime = Date.now();
    
    while (Date.now() - startTime < timeoutMs) {
      const tx = await this.getTransaction(txId);
      
      if (tx.status === 'confirmed' || tx.status === 'failed') {
        return tx;
      }
      
      await new Promise((resolve) => setTimeout(resolve, pollIntervalMs));
    }
    
    throw new Error(`Transaction ${txId} timed out after ${timeoutMs}ms`);
  }
}

/**
 * Create a client instance
 */
export function createClient(config?: ClientConfig): RelyoClient {
  return new RelyoClient(config);
}
