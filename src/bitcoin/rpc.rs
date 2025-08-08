//! Bitcoin Core RPC client implementation

use super::{BitcoinConfig, BitcoinError, BitcoinResult, ConnectionStatus, Network};
use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// Bitcoin Core RPC request
#[derive(Debug, Serialize)]
struct RpcRequest {
    jsonrpc: String,
    method: String,
    params: Value,
    id: u64,
}

/// Bitcoin Core RPC response
#[derive(Debug, Deserialize)]
struct RpcResponse<T> {
    jsonrpc: String,
    result: Option<T>,
    error: Option<RpcError>,
    id: u64,
}

/// RPC error details
#[derive(Debug, Deserialize)]
struct RpcError {
    code: i32,
    message: String,
}

/// Bitcoin Core RPC client
pub struct RpcClient {
    config: BitcoinConfig,
    client: Client,
    request_id: std::sync::atomic::AtomicU64,
}

impl RpcClient {
    /// Create new RPC client
    pub fn new(config: BitcoinConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            config,
            client,
            request_id: std::sync::atomic::AtomicU64::new(1),
        })
    }

    /// Make RPC call to Bitcoin Core
    pub async fn call<T>(&self, method: &str, params: Value) -> BitcoinResult<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let id = self.request_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        
        let request = RpcRequest {
            jsonrpc: "1.0".to_string(),
            method: method.to_string(),
            params: params.clone(),
            id,
        };

        debug!("Bitcoin RPC call: {} with params: {:?}", method, params);

        let mut retries = 0;
        loop {
            match self.make_request(&request).await {
                Ok(response) => return Ok(response),
                Err(e) if retries < self.config.max_retries => {
                    retries += 1;
                    warn!("Bitcoin RPC call failed (attempt {}/{}): {}", retries, self.config.max_retries, e);
                    tokio::time::sleep(Duration::from_millis(1000 * retries as u64)).await;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Make HTTP request to Bitcoin Core
    async fn make_request<T>(&self, request: &RpcRequest) -> BitcoinResult<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let url = if let Some(wallet_name) = &self.config.wallet_name {
            format!("{}/wallet/{}", self.config.rpc_url, wallet_name)
        } else {
            self.config.rpc_url.clone()
        };

        let response = self
            .client
            .post(&url)
            .basic_auth(&self.config.rpc_user, Some(&self.config.rpc_password))
            .json(request)
            .send()
            .await
            .map_err(BitcoinError::Network)?;

        if !response.status().is_success() {
            return Err(BitcoinError::Rpc(format!(
                "HTTP error: {}",
                response.status()
            )));
        }

        let rpc_response: RpcResponse<T> = response
            .json()
            .await
            .map_err(BitcoinError::Network)?;

        if let Some(error) = rpc_response.error {
            return Err(BitcoinError::Rpc(format!(
                "{} (code: {})",
                error.message, error.code
            )));
        }

        rpc_response
            .result
            .ok_or_else(|| BitcoinError::Rpc("No result in RPC response".to_string()))
    }

    /// Get blockchain info
    pub async fn get_blockchain_info(&self) -> BitcoinResult<BlockchainInfo> {
        self.call("getblockchaininfo", json!([])).await
    }

    /// Get network info
    pub async fn get_network_info(&self) -> BitcoinResult<NetworkInfo> {
        self.call("getnetworkinfo", json!([])).await
    }

    /// Get wallet info
    pub async fn get_wallet_info(&self) -> BitcoinResult<WalletInfo> {
        self.call("getwalletinfo", json!([])).await
    }

    /// Get new address
    pub async fn get_new_address(&self, label: Option<&str>, address_type: Option<&str>) -> BitcoinResult<String> {
        let params = match (label, address_type) {
            (Some(l), Some(t)) => json!([l, t]),
            (Some(l), None) => json!([l]),
            (None, Some(t)) => json!(["", t]),
            (None, None) => json!([]),
        };
        
        self.call("getnewaddress", params).await
    }

    /// Get balance
    pub async fn get_balance(&self) -> BitcoinResult<f64> {
        self.call("getbalance", json!([])).await
    }

    /// Get unconfirmed balance
    pub async fn get_unconfirmed_balance(&self) -> BitcoinResult<f64> {
        self.call("getunconfirmedbalance", json!([])).await
    }

    /// List unspent outputs
    pub async fn list_unspent(&self, min_conf: Option<u32>, max_conf: Option<u32>, addresses: Option<Vec<String>>) -> BitcoinResult<Vec<Utxo>> {
        let params = json!([
            min_conf.unwrap_or(1),
            max_conf.unwrap_or(9999999),
            addresses.unwrap_or_default()
        ]);
        
        self.call("listunspent", params).await
    }

    /// Send to address
    pub async fn send_to_address(&self, address: &str, amount: f64, comment: Option<&str>) -> BitcoinResult<String> {
        let params = match comment {
            Some(c) => json!([address, amount, c]),
            None => json!([address, amount]),
        };
        
        self.call("sendtoaddress", params).await
    }

    /// Create raw transaction
    pub async fn create_raw_transaction(&self, inputs: Vec<TxInput>, outputs: Vec<TxOutput>) -> BitcoinResult<String> {
        let params = json!([inputs, outputs]);
        self.call("createrawtransaction", params).await
    }

    /// Sign raw transaction
    pub async fn sign_raw_transaction_with_wallet(&self, hex_string: &str) -> BitcoinResult<SignedTransaction> {
        self.call("signrawtransactionwithwallet", json!([hex_string])).await
    }

    /// Send raw transaction
    pub async fn send_raw_transaction(&self, hex_string: &str) -> BitcoinResult<String> {
        self.call("sendrawtransaction", json!([hex_string])).await
    }

    /// Get transaction
    pub async fn get_transaction(&self, txid: &str) -> BitcoinResult<TransactionInfo> {
        self.call("gettransaction", json!([txid])).await
    }

    /// Estimate smart fee
    pub async fn estimate_smart_fee(&self, conf_target: u32) -> BitcoinResult<FeeEstimateResult> {
        self.call("estimatesmartfee", json!([conf_target])).await
    }

    /// Test connection to Bitcoin Core
    pub async fn test_connection(&self) -> BitcoinResult<ConnectionStatus> {
        let blockchain_info = self.get_blockchain_info().await?;
        let network_info = self.get_network_info().await?;

        Ok(ConnectionStatus {
            connected: true,
            block_height: blockchain_info.blocks,
            network: blockchain_info.chain,
            version: network_info.version.to_string(),
            connections: network_info.connections,
            sync_progress: blockchain_info.verification_progress,
            initial_block_download: blockchain_info.initial_block_download,
        })
    }
}

/// Blockchain information from getblockchaininfo
#[derive(Debug, Deserialize)]
pub struct BlockchainInfo {
    pub chain: String,
    pub blocks: u64,
    pub headers: u64,
    pub bestblockhash: String,
    pub difficulty: f64,
    pub mediantime: u64,
    pub verification_progress: f64,
    pub initial_block_download: bool,
    pub chainwork: String,
    pub size_on_disk: u64,
    pub pruned: bool,
}

/// Network information from getnetworkinfo
#[derive(Debug, Deserialize)]
pub struct NetworkInfo {
    pub version: u32,
    pub subversion: String,
    pub protocol_version: u32,
    pub local_services: String,
    pub local_relay: bool,
    pub time_offset: i32,
    pub connections: u32,
    pub network_active: bool,
    pub networks: Vec<NetworkDetails>,
    pub relay_fee: f64,
    pub incremental_fee: f64,
}

/// Network details
#[derive(Debug, Deserialize)]
pub struct NetworkDetails {
    pub name: String,
    pub limited: bool,
    pub reachable: bool,
    pub proxy: String,
    pub proxy_randomize_credentials: bool,
}

/// Wallet information from getwalletinfo
#[derive(Debug, Deserialize)]
pub struct WalletInfo {
    pub walletname: String,
    pub walletversion: u32,
    pub balance: f64,
    pub unconfirmed_balance: f64,
    pub immature_balance: f64,
    pub txcount: u32,
    pub keypoololdest: u64,
    pub keypoolsize: u32,
    pub hdseedid: Option<String>,
    pub private_keys_enabled: bool,
    pub avoid_reuse: bool,
    pub scanning: Option<ScanningInfo>,
}

/// Scanning information
#[derive(Debug, Deserialize)]
pub struct ScanningInfo {
    pub duration: u32,
    pub progress: f64,
}

/// UTXO from listunspent
#[derive(Debug, Deserialize)]
pub struct Utxo {
    pub txid: String,
    pub vout: u32,
    pub address: String,
    pub label: Option<String>,
    pub script_pubkey: String,
    pub amount: f64,
    pub confirmations: u32,
    pub redeem_script: Option<String>,
    pub witness_script: Option<String>,
    pub spendable: bool,
    pub solvable: bool,
    pub safe: bool,
}

/// Transaction input for createrawtransaction
#[derive(Debug, Serialize)]
pub struct TxInput {
    pub txid: String,
    pub vout: u32,
}

/// Transaction output for createrawtransaction
#[derive(Debug, Serialize)]
pub struct TxOutput {
    pub address: String,
    pub amount: f64,
}

/// Signed transaction result
#[derive(Debug, Deserialize)]
pub struct SignedTransaction {
    pub hex: String,
    pub complete: bool,
    pub errors: Option<Vec<SigningError>>,
}

/// Signing error
#[derive(Debug, Deserialize)]
pub struct SigningError {
    pub txid: String,
    pub vout: u32,
    pub script_sig: String,
    pub sequence: u32,
    pub error: String,
}

/// Transaction information from gettransaction
#[derive(Debug, Deserialize)]
pub struct TransactionInfo {
    pub amount: f64,
    pub fee: Option<f64>,
    pub confirmations: i32,
    pub blockhash: Option<String>,
    pub blockindex: Option<u32>,
    pub blocktime: Option<u64>,
    pub txid: String,
    pub time: u64,
    pub timereceived: u64,
    pub bip125_replaceable: String,
    pub details: Vec<TransactionDetail>,
    pub hex: String,
}

/// Transaction detail
#[derive(Debug, Deserialize)]
pub struct TransactionDetail {
    pub address: String,
    pub category: String,
    pub amount: f64,
    pub label: Option<String>,
    pub vout: u32,
    pub fee: Option<f64>,
    pub abandoned: Option<bool>,
}

/// Fee estimate result
#[derive(Debug, Deserialize)]
pub struct FeeEstimateResult {
    pub feerate: Option<f64>,
    pub errors: Option<Vec<String>>,
    pub blocks: u32,
}

/// Convenient type alias for Bitcoin RPC client
pub type BitcoinRpc = RpcClient;
