//! CLI Commands

use crate::wallet::{Wallet, WalletManager};
use rainsonet_core::{Address, Amount, Nonce};
use rainsonet_relyo::VerifiedTransaction;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// API Client for interacting with RAINSONET node
pub struct ApiClient {
    base_url: String,
    client: Client,
}

impl ApiClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }
    
    /// Get node status
    pub async fn status(&self) -> Result<NodeStatus, ApiError> {
        let url = format!("{}/status", self.base_url);
        let resp: ApiResponse<NodeStatus> = self.client.get(&url).send().await?.json().await?;
        
        if resp.success {
            resp.data.ok_or(ApiError::EmptyResponse)
        } else {
            Err(ApiError::Server(resp.error.unwrap_or_default()))
        }
    }
    
    /// Get account info
    pub async fn get_account(&self, address: &str) -> Result<AccountInfo, ApiError> {
        let url = format!("{}/account/{}", self.base_url, address);
        let resp: ApiResponse<AccountInfo> = self.client.get(&url).send().await?.json().await?;
        
        if resp.success {
            resp.data.ok_or(ApiError::EmptyResponse)
        } else {
            Err(ApiError::Server(resp.error.unwrap_or_default()))
        }
    }
    
    /// Get balance
    pub async fn get_balance(&self, address: &str) -> Result<BalanceInfo, ApiError> {
        let url = format!("{}/balance/{}", self.base_url, address);
        let resp: ApiResponse<BalanceInfo> = self.client.get(&url).send().await?.json().await?;
        
        if resp.success {
            resp.data.ok_or(ApiError::EmptyResponse)
        } else {
            Err(ApiError::Server(resp.error.unwrap_or_default()))
        }
    }
    
    /// Submit transaction
    pub async fn submit_transaction(&self, tx: &TransactionRequest) -> Result<TransactionResponse, ApiError> {
        let url = format!("{}/transaction", self.base_url);
        let resp: ApiResponse<TransactionResponse> = self.client
            .post(&url)
            .json(tx)
            .send()
            .await?
            .json()
            .await?;
        
        if resp.success {
            resp.data.ok_or(ApiError::EmptyResponse)
        } else {
            Err(ApiError::Server(resp.error.unwrap_or_default()))
        }
    }
    
    /// Get transaction status
    pub async fn get_transaction(&self, tx_id: &str) -> Result<TransactionResponse, ApiError> {
        let url = format!("{}/transaction/{}", self.base_url, tx_id);
        let resp: ApiResponse<TransactionResponse> = self.client.get(&url).send().await?.json().await?;
        
        if resp.success {
            resp.data.ok_or(ApiError::EmptyResponse)
        } else {
            Err(ApiError::Server(resp.error.unwrap_or_default()))
        }
    }
}

/// API response wrapper
#[derive(Deserialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

/// Node status
#[derive(Debug, Deserialize)]
pub struct NodeStatus {
    pub node_id: String,
    pub state_version: u64,
    pub state_root: String,
    pub peer_count: usize,
    pub is_validator: bool,
    pub mempool_size: usize,
}

/// Account info
#[derive(Debug, Deserialize)]
pub struct AccountInfo {
    pub address: String,
    pub balance: String,
    pub nonce: u64,
}

/// Balance info
#[derive(Debug, Deserialize)]
pub struct BalanceInfo {
    pub address: String,
    pub balance: String,
    pub balance_relyo: String,
}

/// Transaction request
#[derive(Serialize)]
pub struct TransactionRequest {
    pub from: String,
    pub to: String,
    pub amount: String,
    pub fee: String,
    pub nonce: u64,
    pub public_key: String,
    pub signature: String,
}

/// Transaction response
#[derive(Debug, Deserialize)]
pub struct TransactionResponse {
    pub tx_id: String,
    pub status: String,
}

/// API Error
#[derive(Debug)]
pub enum ApiError {
    Http(reqwest::Error),
    Server(String),
    EmptyResponse,
}

impl From<reqwest::Error> for ApiError {
    fn from(err: reqwest::Error) -> Self {
        ApiError::Http(err)
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::Http(e) => write!(f, "HTTP error: {}", e),
            ApiError::Server(e) => write!(f, "Server error: {}", e),
            ApiError::EmptyResponse => write!(f, "Empty response"),
        }
    }
}

impl std::error::Error for ApiError {}

/// Build transaction request from wallet and parameters
pub fn build_transaction_request(
    wallet: &Wallet,
    to: &str,
    amount: Amount,
    fee: Amount,
    nonce: u64,
) -> Result<TransactionRequest, String> {
    let to_addr = Address::from_hex(to)
        .map_err(|_| "Invalid recipient address")?;
    
    let tx = wallet
        .create_transaction(to_addr, amount, fee, Nonce::new(nonce))
        .map_err(|e| e.to_string())?;
    
    Ok(TransactionRequest {
        from: tx.from.to_hex(),
        to: tx.to.to_hex(),
        amount: tx.amount.0.to_string(),
        fee: tx.fee.0.to_string(),
        nonce: tx.nonce.0,
        public_key: tx.public_key.to_hex(),
        signature: tx.signature.to_hex(),
    })
}
