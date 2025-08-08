//! Wallet data models

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::Chain;

/// Blockchain address
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Address {
    pub address: String,
    pub chain: Chain,
    pub label: Option<String>,
    pub derivation_path: Option<String>, // For HD wallets
    pub balance: Option<Balance>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Address {
    /// Creates a new address
    pub fn new(address: String, chain: &Chain) -> Result<Self> {
        let normalized = Self::normalize_address(&address, chain)?;
        let now = Utc::now();

        Ok(Self {
            address: normalized,
            chain: chain.clone(),
            label: None,
            derivation_path: None,
            balance: None,
            created_at: now,
            updated_at: now,
        })
    }

    /// Creates address with derivation path (for HD wallets)
    pub fn with_derivation(
        address: String,
        chain: &Chain,
        derivation_path: String,
        label: Option<String>,
    ) -> Result<Self> {
        let mut addr = Self::new(address, chain)?;
        addr.derivation_path = Some(derivation_path);
        addr.label = label;
        Ok(addr)
    }

    /// Normalizes address format for the chain
    fn normalize_address(address: &str, chain: &Chain) -> Result<String> {
        let trimmed = address.trim();

        match chain {
            Chain::Ethereum | Chain::BinanceSmartChain | Chain::Polygon => {
                // EVM addresses should be 42 chars (0x + 40 hex)
                if !trimmed.starts_with("0x") {
                    return Err(anyhow::anyhow!("EVM address must start with 0x"));
                }
                if trimmed.len() != 42 {
                    return Err(anyhow::anyhow!("EVM address must be 42 characters"));
                }
                // Convert to lowercase for consistency
                Ok(trimmed.to_lowercase())
            }
            Chain::Bitcoin => {
                // Bitcoin addresses can be various formats
                if trimmed.len() < 26 || trimmed.len() > 62 {
                    return Err(anyhow::anyhow!("Invalid Bitcoin address length"));
                }
                // Basic validation - starts with 1, 3, or bc1
                if !trimmed.starts_with('1')
                    && !trimmed.starts_with('3')
                    && !trimmed.starts_with("bc1")
                {
                    return Err(anyhow::anyhow!("Invalid Bitcoin address format"));
                }
                Ok(trimmed.to_string())
            }
        }
    }

    /// Validates address for specific chain
    pub fn validate_for_chain(&self, chain: &Chain) -> Result<()> {
        if &self.chain != chain {
            return Err(anyhow::anyhow!("Address chain mismatch"));
        }

        // Re-validate the address format
        Self::normalize_address(&self.address, chain)?;
        Ok(())
    }

    /// Updates balance
    pub fn update_balance(&mut self, balance: Balance) {
        self.balance = Some(balance);
        self.updated_at = Utc::now();
    }

    /// Sets label
    pub fn set_label(&mut self, label: String) {
        self.label = Some(label);
        self.updated_at = Utc::now();
    }

    /// Returns display name (label or shortened address)
    pub fn display_name(&self) -> String {
        match &self.label {
            Some(label) => label.clone(),
            None => {
                let addr = &self.address;
                if addr.len() > 10 {
                    format!("{}...{}", &addr[..6], &addr[addr.len() - 4..])
                } else {
                    addr.clone()
                }
            }
        }
    }
}

/// Balance information for an address
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Balance {
    /// Native currency balance (ETH, BNB, MATIC, BTC)
    pub native: String,
    /// Token balances (for EVM chains)
    pub tokens: HashMap<String, TokenBalance>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Block number when balance was fetched
    pub block_number: Option<u64>,
}

impl Balance {
    /// Creates new balance
    pub fn new(native: String) -> Self {
        Self {
            native,
            tokens: HashMap::new(),
            updated_at: Utc::now(),
            block_number: None,
        }
    }

    /// Creates balance with block number
    pub fn with_block(native: String, block_number: u64) -> Self {
        Self {
            native,
            tokens: HashMap::new(),
            updated_at: Utc::now(),
            block_number: Some(block_number),
        }
    }

    /// Adds or updates token balance
    pub fn set_token(&mut self, contract_address: String, balance: TokenBalance) {
        self.tokens.insert(contract_address.to_lowercase(), balance);
        self.updated_at = Utc::now();
    }

    /// Gets token balance
    pub fn get_token(&self, contract_address: &str) -> Option<&TokenBalance> {
        self.tokens.get(&contract_address.to_lowercase())
    }

    /// Returns total USD value if available
    pub fn total_usd_value(&self) -> Option<f64> {
        let mut total = 0.0;
        let mut has_values = false;

        // Add native currency value
        if let Ok(native_val) = self.native.parse::<f64>() {
            // This would need price data - placeholder for now
            total += native_val;
            has_values = true;
        }

        // Add token values
        for token in self.tokens.values() {
            if let Some(usd_value) = token.usd_value {
                total += usd_value;
                has_values = true;
            }
        }

        if has_values {
            Some(total)
        } else {
            None
        }
    }
}

/// Token balance information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenBalance {
    /// Token contract address
    pub contract_address: String,
    /// Token symbol (e.g., USDC, USDT)
    pub symbol: String,
    /// Token name
    pub name: String,
    /// Number of decimals
    pub decimals: u8,
    /// Raw balance (in smallest unit)
    pub raw_balance: String,
    /// Formatted balance (human readable)
    pub formatted_balance: String,
    /// USD value if available
    pub usd_value: Option<f64>,
    /// Price per token in USD
    pub price_usd: Option<f64>,
}

impl TokenBalance {
    /// Creates new token balance
    pub fn new(
        contract_address: String,
        symbol: String,
        name: String,
        decimals: u8,
        raw_balance: String,
    ) -> Self {
        let formatted_balance = Self::format_balance(&raw_balance, decimals);

        Self {
            contract_address: contract_address.to_lowercase(),
            symbol,
            name,
            decimals,
            raw_balance,
            formatted_balance,
            usd_value: None,
            price_usd: None,
        }
    }

    /// Formats raw balance to human readable
    fn format_balance(raw_balance: &str, decimals: u8) -> String {
        if let Ok(raw) = raw_balance.parse::<u128>() {
            let divisor = 10_u128.pow(decimals as u32);
            let whole = raw / divisor;
            let fraction = raw % divisor;

            if fraction == 0 {
                whole.to_string()
            } else {
                let fraction_str = format!("{:0width$}", fraction, width = decimals as usize);
                let trimmed = fraction_str.trim_end_matches('0');
                if trimmed.is_empty() {
                    whole.to_string()
                } else {
                    format!("{}.{}", whole, trimmed)
                }
            }
        } else {
            "0".to_string()
        }
    }

    /// Updates price and calculates USD value
    pub fn update_price(&mut self, price_usd: f64) {
        self.price_usd = Some(price_usd);

        if let Ok(balance) = self.formatted_balance.parse::<f64>() {
            self.usd_value = Some(balance * price_usd);
        }
    }
}

/// Transaction information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub hash: String,
    pub chain: Chain,
    pub from_address: String,
    pub to_address: Option<String>,
    pub value: String,
    pub gas_used: Option<String>,
    pub gas_price: Option<String>,
    pub block_number: Option<u64>,
    pub block_hash: Option<String>,
    pub transaction_index: Option<u32>,
    pub status: TransactionStatus,
    pub timestamp: Option<DateTime<Utc>>,
    pub confirmations: u32,
}

/// Transaction status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransactionStatus {
    Pending,
    Confirmed,
    Failed,
}

/// Sync statistics for a wallet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStats {
    pub wallet_id: Uuid,
    pub last_sync: DateTime<Utc>,
    pub sync_duration_ms: u64,
    pub addresses_synced: u32,
    pub transactions_found: u32,
    pub errors: Vec<String>,
    pub next_sync_at: Option<DateTime<Utc>>,
}

impl SyncStats {
    /// Creates new sync stats
    pub fn new(wallet_id: Uuid) -> Self {
        Self {
            wallet_id,
            last_sync: Utc::now(),
            sync_duration_ms: 0,
            addresses_synced: 0,
            transactions_found: 0,
            errors: Vec::new(),
            next_sync_at: None,
        }
    }

    /// Marks sync as completed
    pub fn complete(&mut self, duration_ms: u64) {
        self.sync_duration_ms = duration_ms;
        self.last_sync = Utc::now();
    }

    /// Adds an error
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }

    /// Schedules next sync
    pub fn schedule_next(&mut self, interval_minutes: i64) {
        self.next_sync_at = Some(Utc::now() + chrono::Duration::minutes(interval_minutes));
    }
}
