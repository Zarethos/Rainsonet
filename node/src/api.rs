//! HTTP API for RAINSONET node

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use rainsonet_core::{Address, Amount, Hash, Nonce};
use rainsonet_relyo::{RelyoTransaction, VerifiedTransaction};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info};

use crate::runtime::NodeRuntime;

/// API state containing node runtime
pub type ApiState = Arc<NodeRuntime>;

/// API response wrapper
#[derive(Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
    
    pub fn err(error: impl ToString) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error.to_string()),
        }
    }
}

/// Balance response
#[derive(Serialize)]
pub struct BalanceResponse {
    pub address: String,
    pub balance: String,
    pub balance_relyo: String,
}

/// Account response
#[derive(Serialize)]
pub struct AccountResponse {
    pub address: String,
    pub balance: String,
    pub nonce: u64,
}

/// Transaction request
#[derive(Deserialize)]
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
#[derive(Serialize)]
pub struct TransactionResponse {
    pub tx_id: String,
    pub status: String,
}

/// Node status response
#[derive(Serialize)]
pub struct NodeStatusResponse {
    pub node_id: String,
    pub state_version: u64,
    pub state_root: String,
    pub peer_count: usize,
    pub is_validator: bool,
    pub mempool_size: usize,
}

/// Create API router
pub fn create_router(state: ApiState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    
    Router::new()
        // Health
        .route("/health", get(health))
        .route("/status", get(status))
        // Accounts
        .route("/account/:address", get(get_account))
        .route("/balance/:address", get(get_balance))
        // Transactions
        .route("/transaction", post(submit_transaction))
        .route("/transaction/:tx_id", get(get_transaction))
        // Mempool
        .route("/mempool", get(get_mempool))
        .with_state(state)
        .layer(cors)
}

/// Health check
async fn health() -> impl IntoResponse {
    Json(serde_json::json!({"status": "ok"}))
}

/// Node status
async fn status(State(runtime): State<ApiState>) -> impl IntoResponse {
    let status = NodeStatusResponse {
        node_id: runtime.node_id().map(|id| id.to_hex()).unwrap_or_default(),
        state_version: runtime.state_version().0,
        state_root: runtime.state_root().to_hex(),
        peer_count: runtime.peer_count(),
        is_validator: runtime.is_validator(),
        mempool_size: runtime.mempool_size(),
    };
    
    Json(ApiResponse::ok(status))
}

/// Get account
async fn get_account(
    State(runtime): State<ApiState>,
    Path(address): Path<String>,
) -> impl IntoResponse {
    match Address::from_hex(&address) {
        Ok(addr) => match runtime.get_account(&addr).await {
            Ok(account) => {
                let response = AccountResponse {
                    address: addr.to_hex(),
                    balance: account.balance.0.to_string(),
                    nonce: account.nonce.0,
                };
                (StatusCode::OK, Json(ApiResponse::ok(response)))
            }
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<AccountResponse>::err(e)),
            ),
        },
        Err(_) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<AccountResponse>::err("Invalid address")),
        ),
    }
}

/// Get balance
async fn get_balance(
    State(runtime): State<ApiState>,
    Path(address): Path<String>,
) -> impl IntoResponse {
    match Address::from_hex(&address) {
        Ok(addr) => match runtime.get_balance(&addr).await {
            Ok(balance) => {
                let balance_relyo = format!(
                    "{}.{}",
                    balance.0 / Amount::ONE_RELYO,
                    balance.0 % Amount::ONE_RELYO
                );
                let response = BalanceResponse {
                    address: addr.to_hex(),
                    balance: balance.0.to_string(),
                    balance_relyo,
                };
                (StatusCode::OK, Json(ApiResponse::ok(response)))
            }
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<BalanceResponse>::err(e)),
            ),
        },
        Err(_) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<BalanceResponse>::err("Invalid address")),
        ),
    }
}

/// Submit transaction
async fn submit_transaction(
    State(runtime): State<ApiState>,
    Json(req): Json<TransactionRequest>,
) -> impl IntoResponse {
    // Parse transaction
    let tx = match parse_transaction_request(&req) {
        Ok(tx) => tx,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<TransactionResponse>::err(e)),
            )
        }
    };
    
    // Verify and submit
    match VerifiedTransaction::new(tx) {
        Ok(verified) => {
            let tx_id = verified.tx_id.to_hex();
            match runtime.submit_transaction(verified).await {
                Ok(_) => {
                    let response = TransactionResponse {
                        tx_id,
                        status: "pending".to_string(),
                    };
                    (StatusCode::ACCEPTED, Json(ApiResponse::ok(response)))
                }
                Err(e) => (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<TransactionResponse>::err(e)),
                ),
            }
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<TransactionResponse>::err(e)),
        ),
    }
}

/// Get transaction status
async fn get_transaction(
    State(runtime): State<ApiState>,
    Path(tx_id): Path<String>,
) -> impl IntoResponse {
    match Hash::from_hex(&tx_id) {
        Ok(id) => {
            let status = if runtime.is_transaction_pending(&id) {
                "pending"
            } else {
                "unknown"
            };
            
            let response = TransactionResponse {
                tx_id,
                status: status.to_string(),
            };
            (StatusCode::OK, Json(ApiResponse::ok(response)))
        }
        Err(_) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<TransactionResponse>::err("Invalid transaction ID")),
        ),
    }
}

/// Get mempool
async fn get_mempool(State(runtime): State<ApiState>) -> impl IntoResponse {
    let tx_ids: Vec<String> = runtime
        .mempool_tx_ids()
        .iter()
        .map(|id| id.to_hex())
        .collect();
    
    Json(ApiResponse::ok(tx_ids))
}

fn parse_transaction_request(req: &TransactionRequest) -> Result<RelyoTransaction, String> {
    let from = Address::from_hex(&req.from).map_err(|_| "Invalid from address")?;
    let to = Address::from_hex(&req.to).map_err(|_| "Invalid to address")?;
    let amount = Amount::new(
        req.amount
            .parse::<u128>()
            .map_err(|_| "Invalid amount")?,
    );
    let fee = Amount::new(req.fee.parse::<u128>().map_err(|_| "Invalid fee")?);
    let nonce = Nonce::new(req.nonce);
    
    let public_key_bytes = hex::decode(&req.public_key).map_err(|_| "Invalid public key")?;
    if public_key_bytes.len() != 32 {
        return Err("Public key must be 32 bytes".into());
    }
    let mut pk_arr = [0u8; 32];
    pk_arr.copy_from_slice(&public_key_bytes);
    let public_key = rainsonet_core::PublicKey::from_bytes(pk_arr);
    
    let sig_bytes = hex::decode(&req.signature).map_err(|_| "Invalid signature")?;
    if sig_bytes.len() != 64 {
        return Err("Signature must be 64 bytes".into());
    }
    let mut sig_arr = [0u8; 64];
    sig_arr.copy_from_slice(&sig_bytes);
    let signature = rainsonet_core::Signature::from_bytes(sig_arr);
    
    Ok(RelyoTransaction {
        from,
        to,
        amount,
        fee,
        nonce,
        timestamp: rainsonet_core::Timestamp::now(),
        public_key,
        signature,
    })
}

/// Start API server
pub async fn start_api_server(runtime: Arc<NodeRuntime>, listen_addr: &str) -> anyhow::Result<()> {
    let router = create_router(runtime);
    
    let listener = tokio::net::TcpListener::bind(listen_addr).await?;
    info!("API server listening on {}", listen_addr);
    
    axum::serve(listener, router).await?;
    
    Ok(())
}
