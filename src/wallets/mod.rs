//! Multi-Wallet Core Module
//!
//! Obsługuje 200+ portfeli z cold signing i multi-chain support

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

pub mod cache;
pub mod manager;
pub mod models;
pub mod sync;

pub use manager::WalletManager;
pub use models::*;

/// Supported blockchain networks
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Chain {
    Ethereum,
    BinanceSmartChain,
    Polygon,
    Bitcoin,
}

impl Chain {
    /// Returns the chain ID for EVM chains
    pub fn chain_id(&self) -> Option<u64> {
        match self {
            Chain::Ethereum => Some(1),
            Chain::BinanceSmartChain => Some(56),
            Chain::Polygon => Some(137),
            Chain::Bitcoin => None,
        }
    }

    /// Returns the native currency symbol
    pub fn native_currency(&self) -> &'static str {
        match self {
            Chain::Ethereum => "ETH",
            Chain::BinanceSmartChain => "BNB",
            Chain::Polygon => "MATIC",
            Chain::Bitcoin => "BTC",
        }
    }

    /// Returns RPC endpoint URL
    pub fn rpc_url(&self) -> &'static str {
        match self {
            Chain::Ethereum => "https://eth-mainnet.g.alchemy.com/v2/",
            Chain::BinanceSmartChain => "https://bsc-dataseed.binance.org/",
            Chain::Polygon => "https://polygon-rpc.com/",
            Chain::Bitcoin => "https://blockstream.info/api/",
        }
    }

    /// Returns block confirmation count for finality
    pub fn confirmation_blocks(&self) -> u32 {
        match self {
            Chain::Ethereum => 12,
            Chain::BinanceSmartChain => 15,
            Chain::Polygon => 20,
            Chain::Bitcoin => 6,
        }
    }
}

/// Wallet type for different signing methods
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WalletType {
    /// Watch-only wallet (no private keys)
    WatchOnly,
    /// Cold wallet with offline signing
    Cold,
    /// Hardware wallet
    Hardware,
}

/// Wallet status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WalletStatus {
    Active,
    Inactive,
    Syncing,
    Error(String),
}

/// Main wallet structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wallet {
    pub id: Uuid,
    pub name: String,
    pub wallet_type: WalletType,
    pub chain: Chain,
    pub status: WalletStatus,
    pub addresses: Vec<Address>,
    pub xpub: Option<String>, // For BTC hierarchical wallets
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_sync: Option<DateTime<Utc>>,
    pub metadata: HashMap<String, String>,
}

impl Wallet {
    /// Creates a new wallet
    pub fn new(
        name: String,
        wallet_type: WalletType,
        chain: Chain,
        addresses: Vec<Address>,
        xpub: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            wallet_type,
            chain,
            status: WalletStatus::Active,
            addresses,
            xpub,
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
            last_sync: None,
            metadata: HashMap::new(),
        }
    }

    /// Validates wallet configuration
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            return Err(anyhow::anyhow!("Wallet name cannot be empty"));
        }

        if self.addresses.is_empty() && self.xpub.is_none() {
            return Err(anyhow::anyhow!("Wallet must have addresses or xpub"));
        }

        // Validate addresses for the chain
        for address in &self.addresses {
            address.validate_for_chain(&self.chain)?;
        }

        // Validate xpub for Bitcoin
        if let Some(ref xpub) = self.xpub {
            if self.chain != Chain::Bitcoin {
                return Err(anyhow::anyhow!("XPub only supported for Bitcoin"));
            }
            self.validate_xpub(xpub)?;
        }

        Ok(())
    }

    /// Validates Bitcoin xpub format
    fn validate_xpub(&self, xpub: &str) -> Result<()> {
        if !xpub.starts_with("xpub") && !xpub.starts_with("ypub") && !xpub.starts_with("zpub") {
            return Err(anyhow::anyhow!("Invalid xpub format"));
        }

        if xpub.len() < 100 || xpub.len() > 120 {
            return Err(anyhow::anyhow!("Invalid xpub length"));
        }

        Ok(())
    }

    /// Updates wallet status
    pub fn update_status(&mut self, status: WalletStatus) {
        self.status = status;
        self.updated_at = Utc::now();
    }

    /// Marks wallet as synced
    pub fn mark_synced(&mut self) {
        self.last_sync = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Adds a tag to the wallet
    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
            self.updated_at = Utc::now();
        }
    }

    /// Removes a tag from the wallet
    pub fn remove_tag(&mut self, tag: &str) {
        if let Some(pos) = self.tags.iter().position(|t| t == tag) {
            self.tags.remove(pos);
            self.updated_at = Utc::now();
        }
    }

    /// Sets metadata value
    pub fn set_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
        self.updated_at = Utc::now();
    }

    /// Gets metadata value
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    /// Returns primary address
    pub fn primary_address(&self) -> Option<&Address> {
        self.addresses.first()
    }

    /// Checks if wallet needs sync (older than threshold)
    pub fn needs_sync(&self, threshold_minutes: i64) -> bool {
        match self.last_sync {
            Some(last_sync) => {
                let threshold = chrono::Duration::minutes(threshold_minutes);
                Utc::now() - last_sync > threshold
            }
            None => true,
        }
    }
}

/// Wallet creation request
#[derive(Debug, Deserialize)]
pub struct CreateWalletRequest {
    pub name: String,
    pub wallet_type: WalletType,
    pub chain: Chain,
    pub addresses: Vec<String>,
    pub xpub: Option<String>,
    pub tags: Option<Vec<String>>,
    pub metadata: Option<HashMap<String, String>>,
}

impl CreateWalletRequest {
    /// Converts to Wallet
    pub fn into_wallet(self) -> Result<Wallet> {
        let addresses: Result<Vec<Address>> = self
            .addresses
            .into_iter()
            .map(|addr| Address::new(addr, &self.chain))
            .collect();

        let mut wallet = Wallet::new(
            self.name,
            self.wallet_type,
            self.chain,
            addresses?,
            self.xpub,
        );

        if let Some(tags) = self.tags {
            wallet.tags = tags;
        }

        if let Some(metadata) = self.metadata {
            wallet.metadata = metadata;
        }

        wallet.validate()?;
        Ok(wallet)
    }
}

/// Wallet update request
#[derive(Debug, Deserialize)]
pub struct UpdateWalletRequest {
    pub name: Option<String>,
    pub status: Option<WalletStatus>,
    pub tags: Option<Vec<String>>,
    pub metadata: Option<HashMap<String, String>>,
}

/// Wallet list filters
#[derive(Debug, Deserialize)]
pub struct WalletFilters {
    pub chain: Option<Chain>,
    pub wallet_type: Option<WalletType>,
    pub status: Option<WalletStatus>,
    pub tags: Option<Vec<String>>,
    pub name_contains: Option<String>,
    pub needs_sync: Option<bool>,
}

/// Pagination parameters
#[derive(Debug, Deserialize)]
pub struct Pagination {
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            page: Some(1),
            limit: Some(50),
        }
    }
}

/// Wallet list response
#[derive(Debug, Serialize)]
pub struct WalletListResponse {
    pub wallets: Vec<Wallet>,
    pub total: u64,
    pub page: u32,
    pub limit: u32,
    pub total_pages: u32,
}
