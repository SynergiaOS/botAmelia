//! Bitcoin indexer using Blockstream API

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

use super::{ChainHealth, IndexerConfig};
use crate::wallets::models::*;

/// Bitcoin indexer
pub struct BitcoinIndexer {
    config: IndexerConfig,
    client: Client,
    base_url: String,
}

/// Bitcoin address info from Blockstream API
#[derive(Debug, Deserialize)]
struct AddressInfo {
    address: String,
    chain_stats: ChainStats,
    mempool_stats: MempoolStats,
}

#[derive(Debug, Deserialize)]
struct ChainStats {
    funded_txo_count: u64,
    funded_txo_sum: u64,
    spent_txo_count: u64,
    spent_txo_sum: u64,
    tx_count: u64,
}

#[derive(Debug, Deserialize)]
struct MempoolStats {
    funded_txo_count: u64,
    funded_txo_sum: u64,
    spent_txo_count: u64,
    spent_txo_sum: u64,
    tx_count: u64,
}

/// Bitcoin block info
#[derive(Debug, Deserialize)]
pub struct BlockInfo {
    id: String,
    height: u64,
    version: u32,
    timestamp: u64,
    tx_count: u32,
    size: u64,
    weight: u64,
    merkle_root: String,
    previousblockhash: Option<String>,
    mediantime: u64,
    nonce: u64,
    bits: u32,
    difficulty: f64,
}

impl BitcoinIndexer {
    /// Creates new Bitcoin indexer
    pub async fn new(config: IndexerConfig, client: Client) -> Result<Self> {
        let base_url = config
            .rpc_urls
            .get(&crate::wallets::Chain::Bitcoin)
            .cloned()
            .unwrap_or_else(|| "https://blockstream.info/api".to_string());

        info!("Bitcoin indexer initialized with base URL: {}", base_url);
        Ok(Self {
            config,
            client,
            base_url,
        })
    }

    /// Syncs balances for Bitcoin addresses
    pub async fn sync_addresses(&self, addresses: &[String]) -> Result<Vec<Balance>> {
        let mut balances = Vec::new();

        // Get current block height
        let current_block = self.get_current_block().await?;

        // Process addresses in chunks
        for chunk in addresses.chunks(self.config.batch_size) {
            let chunk_balances = self.sync_address_chunk(chunk, current_block).await?;
            balances.extend(chunk_balances);
        }

        Ok(balances)
    }

    /// Syncs a chunk of addresses
    async fn sync_address_chunk(
        &self,
        addresses: &[String],
        block_height: u64,
    ) -> Result<Vec<Balance>> {
        let mut balances = Vec::new();

        for address in addresses {
            match self.get_address_balance(address, block_height).await {
                Ok(balance) => balances.push(balance),
                Err(e) => {
                    warn!("Failed to get Bitcoin balance for {}: {}", address, e);
                    // Create empty balance as fallback
                    balances.push(Balance::with_block("0".to_string(), block_height));
                }
            }
        }

        Ok(balances)
    }

    /// Gets balance for a single Bitcoin address
    async fn get_address_balance(&self, address: &str, block_height: u64) -> Result<Balance> {
        let url = format!("{}/address/{}", self.base_url, address);

        let response: AddressInfo = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch address info")?
            .json()
            .await
            .context("Failed to parse address info")?;

        // Calculate balance (funded - spent)
        let balance_satoshis =
            response.chain_stats.funded_txo_sum - response.chain_stats.spent_txo_sum;

        // Add mempool balance
        let mempool_balance =
            response.mempool_stats.funded_txo_sum - response.mempool_stats.spent_txo_sum;
        let total_balance_satoshis = balance_satoshis + mempool_balance;

        // Convert satoshis to BTC
        let balance_btc = total_balance_satoshis as f64 / 100_000_000.0;

        let balance = Balance::with_block(balance_btc.to_string(), block_height);

        debug!("Bitcoin balance for {}: {} BTC", address, balance_btc);
        Ok(balance)
    }

    /// Gets current Bitcoin block height
    pub async fn get_current_block(&self) -> Result<u64> {
        let url = format!("{}/blocks/tip/height", self.base_url);

        let height: u64 = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch block height")?
            .text()
            .await
            .context("Failed to get block height text")?
            .parse()
            .context("Failed to parse block height")?;

        Ok(height)
    }

    /// Gets Bitcoin block info
    pub async fn get_block_info(&self, height: u64) -> Result<BlockInfo> {
        let url = format!("{}/block-height/{}", self.base_url, height);

        // First get block hash
        let block_hash: String = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch block hash")?
            .text()
            .await
            .context("Failed to get block hash text")?;

        // Then get block info
        let url = format!("{}/block/{}", self.base_url, block_hash);
        let block_info: BlockInfo = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch block info")?
            .json()
            .await
            .context("Failed to parse block info")?;

        Ok(block_info)
    }

    /// Gets transaction history for address
    pub async fn get_address_transactions(&self, address: &str) -> Result<Vec<Transaction>> {
        let url = format!("{}/address/{}/txs", self.base_url, address);

        let txs: Vec<serde_json::Value> = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch transactions")?
            .json()
            .await
            .context("Failed to parse transactions")?;

        let mut transactions = Vec::new();

        for tx in txs {
            if let Some(transaction) = self.parse_transaction(tx, address).await? {
                transactions.push(transaction);
            }
        }

        Ok(transactions)
    }

    /// Parses transaction from API response
    async fn parse_transaction(
        &self,
        tx: serde_json::Value,
        address: &str,
    ) -> Result<Option<Transaction>> {
        let txid = tx["txid"].as_str().unwrap_or_default().to_string();
        let status = tx["status"].as_object();

        let (block_height, confirmed) = if let Some(status) = status {
            let height = status["block_height"].as_u64().unwrap_or(0);
            let confirmed = status["confirmed"].as_bool().unwrap_or(false);
            (Some(height), confirmed)
        } else {
            (None, false)
        };

        // Calculate value for this address
        let mut value = 0i64;

        // Check inputs (spending from this address)
        if let Some(vin) = tx["vin"].as_array() {
            for input in vin {
                if let Some(prevout) = input["prevout"].as_object() {
                    if prevout["scriptpubkey_address"].as_str() == Some(address) {
                        value -= prevout["value"].as_i64().unwrap_or(0);
                    }
                }
            }
        }

        // Check outputs (receiving to this address)
        if let Some(vout) = tx["vout"].as_array() {
            for output in vout {
                if output["scriptpubkey_address"].as_str() == Some(address) {
                    value += output["value"].as_i64().unwrap_or(0);
                }
            }
        }

        // Only include transactions that affect this address
        if value == 0 {
            return Ok(None);
        }

        let transaction = Transaction {
            hash: txid,
            chain: crate::wallets::Chain::Bitcoin,
            from_address: "".to_string(), // Bitcoin has multiple inputs
            to_address: Some(address.to_string()),
            value: value.to_string(),
            gas_used: None,
            gas_price: None,
            block_number: block_height,
            block_hash: None,
            transaction_index: None,
            status: if confirmed {
                TransactionStatus::Confirmed
            } else {
                TransactionStatus::Pending
            },
            timestamp: None,
            confirmations: 0, // Would need to calculate
        };

        Ok(Some(transaction))
    }

    /// Health check for Bitcoin indexer
    pub async fn health_check(&self) -> Result<ChainHealth> {
        let start_time = std::time::Instant::now();

        match self.get_current_block().await {
            Ok(block_height) => {
                let latency_ms = start_time.elapsed().as_millis() as u64;
                Ok(ChainHealth {
                    connected: true,
                    latest_block: block_height,
                    latency_ms,
                    error: None,
                })
            }
            Err(e) => Ok(ChainHealth {
                connected: false,
                latest_block: 0,
                latency_ms: start_time.elapsed().as_millis() as u64,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Validates Bitcoin address format
    pub fn validate_address(&self, address: &str) -> bool {
        // Basic Bitcoin address validation
        if address.len() < 26 || address.len() > 62 {
            return false;
        }

        // Check prefixes
        address.starts_with('1') || address.starts_with('3') || address.starts_with("bc1")
    }
}
