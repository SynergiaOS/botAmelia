//! EVM blockchain indexer for Ethereum, BSC, Polygon

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

use super::{ChainHealth, IndexerConfig};
use crate::wallets::{models::*, Chain};

/// EVM RPC request
#[derive(Debug, Serialize)]
struct RpcRequest {
    jsonrpc: String,
    method: String,
    params: Value,
    id: u64,
}

/// EVM RPC response
#[derive(Debug, Deserialize)]
struct RpcResponse<T> {
    jsonrpc: String,
    result: Option<T>,
    error: Option<RpcError>,
    id: u64,
}

/// RPC error
#[derive(Debug, Deserialize)]
struct RpcError {
    code: i32,
    message: String,
}

/// EVM indexer for Ethereum-compatible chains
pub struct EvmIndexer {
    config: IndexerConfig,
    http_client: Client,
}

impl EvmIndexer {
    /// Creates new EVM indexer
    pub async fn new(config: IndexerConfig, http_client: Client) -> Result<Self> {
        info!("EVM indexer initialized");
        Ok(Self {
            config,
            http_client,
        })
    }

    /// Syncs balances for addresses on EVM chain
    pub async fn sync_addresses(
        &self,
        chain: &Chain,
        addresses: &[String],
    ) -> Result<Vec<Balance>> {
        let rpc_url = self
            .config
            .rpc_urls
            .get(chain)
            .ok_or_else(|| anyhow::anyhow!("No RPC URL configured for {:?}", chain))?;

        let current_block = self.get_current_block(chain).await?;
        let mut balances = Vec::new();

        // Batch process addresses
        for chunk in addresses.chunks(self.config.batch_size) {
            let chunk_balances = self
                .sync_address_batch(rpc_url, chunk, current_block)
                .await?;
            balances.extend(chunk_balances);
        }

        info!("Synced {} addresses on {:?}", addresses.len(), chain);
        Ok(balances)
    }

    /// Syncs a batch of addresses
    async fn sync_address_batch(
        &self,
        rpc_url: &str,
        addresses: &[String],
        block_number: u64,
    ) -> Result<Vec<Balance>> {
        let mut balances = Vec::new();

        for address in addresses {
            match self
                .get_address_balance(rpc_url, address, block_number)
                .await
            {
                Ok(balance) => balances.push(balance),
                Err(e) => {
                    warn!("Failed to get balance for {}: {}", address, e);
                    // Create empty balance as fallback
                    balances.push(Balance::with_block("0".to_string(), block_number));
                }
            }
        }

        Ok(balances)
    }

    /// Gets balance for a single address
    async fn get_address_balance(
        &self,
        rpc_url: &str,
        address: &str,
        block_number: u64,
    ) -> Result<Balance> {
        // Get native balance
        let native_balance = self
            .get_native_balance(rpc_url, address, block_number)
            .await?;

        let mut balance = Balance::with_block(native_balance, block_number);

        // Get token balances (simplified - would need token contract addresses)
        // For now, just return native balance

        Ok(balance)
    }

    /// Gets native ETH/BNB/MATIC balance
    async fn get_native_balance(
        &self,
        rpc_url: &str,
        address: &str,
        block_number: u64,
    ) -> Result<String> {
        let request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "eth_getBalance".to_string(),
            params: json!([address, format!("0x{:x}", block_number)]),
            id: 1,
        };

        let response: RpcResponse<String> = self
            .http_client
            .post(rpc_url)
            .json(&request)
            .send()
            .await
            .context("Failed to send RPC request")?
            .json()
            .await
            .context("Failed to parse RPC response")?;

        if let Some(error) = response.error {
            return Err(anyhow::anyhow!(
                "RPC error: {} ({})",
                error.message,
                error.code
            ));
        }

        let hex_balance = response
            .result
            .ok_or_else(|| anyhow::anyhow!("No result in RPC response"))?;

        // Convert hex to decimal
        let balance_wei =
            u128::from_str_radix(&hex_balance[2..], 16).context("Failed to parse hex balance")?;

        // Convert wei to ETH (18 decimals)
        let balance_eth = balance_wei as f64 / 1e18;

        Ok(balance_eth.to_string())
    }

    /// Gets current block number
    pub async fn get_current_block(&self, chain: &Chain) -> Result<u64> {
        let rpc_url = self
            .config
            .rpc_urls
            .get(chain)
            .ok_or_else(|| anyhow::anyhow!("No RPC URL configured for {:?}", chain))?;

        let request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "eth_blockNumber".to_string(),
            params: json!([]),
            id: 1,
        };

        let response: RpcResponse<String> = self
            .http_client
            .post(rpc_url)
            .json(&request)
            .send()
            .await
            .context("Failed to send RPC request")?
            .json()
            .await
            .context("Failed to parse RPC response")?;

        if let Some(error) = response.error {
            return Err(anyhow::anyhow!(
                "RPC error: {} ({})",
                error.message,
                error.code
            ));
        }

        let hex_block = response
            .result
            .ok_or_else(|| anyhow::anyhow!("No result in RPC response"))?;

        let block_number =
            u64::from_str_radix(&hex_block[2..], 16).context("Failed to parse hex block number")?;

        Ok(block_number)
    }

    /// Gets token balance for ERC-20 token
    pub async fn get_token_balance(
        &self,
        rpc_url: &str,
        token_address: &str,
        wallet_address: &str,
        block_number: u64,
    ) -> Result<TokenBalance> {
        // ERC-20 balanceOf function signature
        let function_sig = "0x70a08231"; // balanceOf(address)
        let padded_address = format!("{:0>64}", &wallet_address[2..]);
        let data = format!("{}{}", function_sig, padded_address);

        let request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "eth_call".to_string(),
            params: json!([{
                "to": token_address,
                "data": data
            }, format!("0x{:x}", block_number)]),
            id: 1,
        };

        let response: RpcResponse<String> = self
            .http_client
            .post(rpc_url)
            .json(&request)
            .send()
            .await
            .context("Failed to send token balance RPC request")?
            .json()
            .await
            .context("Failed to parse token balance RPC response")?;

        if let Some(error) = response.error {
            return Err(anyhow::anyhow!(
                "RPC error: {} ({})",
                error.message,
                error.code
            ));
        }

        let hex_balance = response
            .result
            .ok_or_else(|| anyhow::anyhow!("No result in token balance RPC response"))?;

        let raw_balance = if hex_balance.len() > 2 {
            u128::from_str_radix(&hex_balance[2..], 16)
                .context("Failed to parse hex token balance")?
                .to_string()
        } else {
            "0".to_string()
        };

        // For now, create a generic token balance
        // In production, you'd fetch token metadata (symbol, name, decimals)
        Ok(TokenBalance::new(
            token_address.to_string(),
            "UNKNOWN".to_string(),
            "Unknown Token".to_string(),
            18, // Default to 18 decimals
            raw_balance,
        ))
    }

    /// Health check for EVM chain
    pub async fn health_check(&self, chain: &Chain) -> Result<ChainHealth> {
        let start_time = std::time::Instant::now();

        match self.get_current_block(chain).await {
            Ok(block_number) => {
                let latency_ms = start_time.elapsed().as_millis() as u64;
                Ok(ChainHealth {
                    connected: true,
                    latest_block: block_number,
                    latency_ms,
                    error: None,
                })
            }
            Err(e) => {
                let latency_ms = start_time.elapsed().as_millis() as u64;
                Ok(ChainHealth {
                    connected: false,
                    latest_block: 0,
                    latency_ms,
                    error: Some(e.to_string()),
                })
            }
        }
    }

    /// Gets transaction history for address
    pub async fn get_transaction_history(
        &self,
        chain: &Chain,
        address: &str,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<Transaction>> {
        // This would implement transaction history fetching
        // For now, return empty vector
        Ok(Vec::new())
    }

    /// Estimates gas for transaction
    pub async fn estimate_gas(
        &self,
        chain: &Chain,
        from: &str,
        to: &str,
        value: &str,
        data: Option<&str>,
    ) -> Result<u64> {
        let rpc_url = self
            .config
            .rpc_urls
            .get(chain)
            .ok_or_else(|| anyhow::anyhow!("No RPC URL configured for {:?}", chain))?;

        let mut tx_object = json!({
            "from": from,
            "to": to,
            "value": value
        });

        if let Some(data) = data {
            tx_object["data"] = json!(data);
        }

        let request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "eth_estimateGas".to_string(),
            params: json!([tx_object]),
            id: 1,
        };

        let response: RpcResponse<String> = self
            .http_client
            .post(rpc_url)
            .json(&request)
            .send()
            .await
            .context("Failed to send gas estimation request")?
            .json()
            .await
            .context("Failed to parse gas estimation response")?;

        if let Some(error) = response.error {
            return Err(anyhow::anyhow!(
                "RPC error: {} ({})",
                error.message,
                error.code
            ));
        }

        let hex_gas = response
            .result
            .ok_or_else(|| anyhow::anyhow!("No result in gas estimation response"))?;

        let gas_limit =
            u64::from_str_radix(&hex_gas[2..], 16).context("Failed to parse hex gas limit")?;

        Ok(gas_limit)
    }
}
