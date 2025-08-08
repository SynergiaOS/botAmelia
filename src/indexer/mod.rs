//! Blockchain indexer for multi-chain wallet synchronization

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use crate::wallets::{models::*, Chain};

pub mod bitcoin;
pub mod coinstats;
pub mod evm;

/// Indexer configuration
#[derive(Debug, Clone)]
pub struct IndexerConfig {
    pub rpc_urls: HashMap<Chain, String>,
    pub api_keys: HashMap<String, String>,
    pub batch_size: usize,
    pub max_concurrent: usize,
    pub timeout_seconds: u64,
}

impl Default for IndexerConfig {
    fn default() -> Self {
        let mut rpc_urls = HashMap::new();
        rpc_urls.insert(
            Chain::Ethereum,
            "https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY".to_string(),
        );
        rpc_urls.insert(
            Chain::BinanceSmartChain,
            "https://bsc-dataseed.binance.org/".to_string(),
        );
        rpc_urls.insert(Chain::Polygon, "https://polygon-rpc.com/".to_string());
        rpc_urls.insert(Chain::Bitcoin, "https://blockstream.info/api/".to_string());

        let mut api_keys = HashMap::new();
        api_keys.insert("alchemy".to_string(), "YOUR_ALCHEMY_KEY".to_string());
        api_keys.insert("coinstats".to_string(), "YOUR_COINSTATS_KEY".to_string());

        Self {
            rpc_urls,
            api_keys,
            batch_size: 50,
            max_concurrent: 10,
            timeout_seconds: 30,
        }
    }
}

/// Main indexer for all chains
pub struct MultiChainIndexer {
    config: IndexerConfig,
    http_client: Client,
    evm_indexer: evm::EvmIndexer,
    bitcoin_indexer: bitcoin::BitcoinIndexer,
    coinstats_client: coinstats::CoinStatsClient,
    sync_state: Arc<RwLock<HashMap<Chain, SyncState>>>,
}

/// Sync state for a chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncState {
    pub chain: Chain,
    pub last_block: u64,
    pub last_sync: chrono::DateTime<chrono::Utc>,
    pub sync_duration_ms: u64,
    pub addresses_synced: u32,
    pub errors: Vec<String>,
}

impl SyncState {
    pub fn new(chain: Chain) -> Self {
        Self {
            chain,
            last_block: 0,
            last_sync: chrono::Utc::now(),
            sync_duration_ms: 0,
            addresses_synced: 0,
            errors: Vec::new(),
        }
    }
}

impl MultiChainIndexer {
    /// Creates new multi-chain indexer
    pub async fn new(config: IndexerConfig) -> Result<Self> {
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()
            .context("Failed to create HTTP client")?;

        let evm_indexer = evm::EvmIndexer::new(config.clone(), http_client.clone()).await?;

        let bitcoin_indexer =
            bitcoin::BitcoinIndexer::new(config.clone(), http_client.clone()).await?;

        let coinstats_client = coinstats::CoinStatsClient::new(
            config
                .api_keys
                .get("coinstats")
                .cloned()
                .unwrap_or_default(),
            http_client.clone(),
        )
        .await?;

        let sync_state = Arc::new(RwLock::new(HashMap::new()));

        info!("MultiChainIndexer initialized");
        Ok(Self {
            config,
            http_client,
            evm_indexer,
            bitcoin_indexer,
            coinstats_client,
            sync_state,
        })
    }

    /// Syncs balances for a list of addresses on specific chain
    pub async fn sync_addresses(
        &self,
        chain: &Chain,
        addresses: &[String],
    ) -> Result<Vec<Balance>> {
        let start_time = std::time::Instant::now();

        info!("Syncing {} addresses on {:?}", addresses.len(), chain);

        let balances = match chain {
            Chain::Ethereum | Chain::BinanceSmartChain | Chain::Polygon => {
                self.evm_indexer.sync_addresses(chain, addresses).await?
            }
            Chain::Bitcoin => self.bitcoin_indexer.sync_addresses(addresses).await?,
        };

        // Update sync state
        let mut state = self.sync_state.write().await;
        let sync_state = state
            .entry(chain.clone())
            .or_insert_with(|| SyncState::new(chain.clone()));
        sync_state.last_sync = chrono::Utc::now();
        sync_state.sync_duration_ms = start_time.elapsed().as_millis() as u64;
        sync_state.addresses_synced = addresses.len() as u32;

        info!(
            "Synced {} addresses on {:?} in {}ms",
            addresses.len(),
            chain,
            sync_state.sync_duration_ms
        );

        Ok(balances)
    }

    /// Gets current block number for chain
    pub async fn get_current_block(&self, chain: &Chain) -> Result<u64> {
        match chain {
            Chain::Ethereum | Chain::BinanceSmartChain | Chain::Polygon => {
                self.evm_indexer.get_current_block(chain).await
            }
            Chain::Bitcoin => self.bitcoin_indexer.get_current_block().await,
        }
    }

    /// Gets token prices from CoinStats
    pub async fn get_token_prices(&self, symbols: &[String]) -> Result<HashMap<String, f64>> {
        self.coinstats_client.get_prices(symbols).await
    }

    /// Gets portfolio valuation
    pub async fn get_portfolio_value(&self, balances: &[Balance]) -> Result<f64> {
        let mut total_value = 0.0;
        let mut symbols = Vec::new();

        // Collect all symbols
        for balance in balances {
            symbols.push("ETH".to_string()); // Native currency placeholder
            for token in balance.tokens.values() {
                symbols.push(token.symbol.clone());
            }
        }

        // Get prices
        let prices = self.get_token_prices(&symbols).await?;

        // Calculate total value
        for balance in balances {
            // Native currency value
            if let Ok(native_amount) = balance.native.parse::<f64>() {
                if let Some(price) = prices.get("ETH") {
                    // Placeholder
                    total_value += native_amount * price;
                }
            }

            // Token values
            for token in balance.tokens.values() {
                if let Ok(token_amount) = token.formatted_balance.parse::<f64>() {
                    if let Some(price) = prices.get(&token.symbol) {
                        total_value += token_amount * price;
                    }
                }
            }
        }

        Ok(total_value)
    }

    /// Gets sync statistics
    pub async fn get_sync_stats(&self) -> Result<HashMap<Chain, SyncState>> {
        let state = self.sync_state.read().await;
        Ok(state.clone())
    }

    /// Health check for all indexers
    pub async fn health_check(&self) -> Result<IndexerHealth> {
        let mut health = IndexerHealth {
            healthy: true,
            chains: HashMap::new(),
            errors: Vec::new(),
        };

        // Check EVM chains
        for chain in [Chain::Ethereum, Chain::BinanceSmartChain, Chain::Polygon] {
            match self.evm_indexer.health_check(&chain).await {
                Ok(chain_health) => {
                    health.chains.insert(chain, chain_health);
                }
                Err(e) => {
                    health.healthy = false;
                    health.errors.push(format!("{:?}: {}", chain, e));
                    health.chains.insert(
                        chain,
                        ChainHealth {
                            connected: false,
                            latest_block: 0,
                            latency_ms: 0,
                            error: Some(e.to_string()),
                        },
                    );
                }
            }
        }

        // Check Bitcoin
        match self.bitcoin_indexer.health_check().await {
            Ok(chain_health) => {
                health.chains.insert(Chain::Bitcoin, chain_health);
            }
            Err(e) => {
                health.healthy = false;
                health.errors.push(format!("Bitcoin: {}", e));
                health.chains.insert(
                    Chain::Bitcoin,
                    ChainHealth {
                        connected: false,
                        latest_block: 0,
                        latency_ms: 0,
                        error: Some(e.to_string()),
                    },
                );
            }
        }

        // Check CoinStats
        match self.coinstats_client.health_check().await {
            Ok(_) => {}
            Err(e) => {
                health.healthy = false;
                health.errors.push(format!("CoinStats: {}", e));
            }
        }

        Ok(health)
    }
}

/// Overall indexer health
#[derive(Debug, Serialize)]
pub struct IndexerHealth {
    pub healthy: bool,
    pub chains: HashMap<Chain, ChainHealth>,
    pub errors: Vec<String>,
}

/// Health status for a specific chain
#[derive(Debug, Serialize)]
pub struct ChainHealth {
    pub connected: bool,
    pub latest_block: u64,
    pub latency_ms: u64,
    pub error: Option<String>,
}
