//! Bitcoin Core integration module
//! 
//! This module provides comprehensive Bitcoin Core RPC integration,
//! including wallet management, transaction handling, and PSBT support.

pub mod core;
pub mod hardware_signer;
pub mod key_manager;
pub mod psbt;
pub mod psbt_advanced;
pub mod rpc;
pub mod script_types;
pub mod security_validator;
pub mod transaction;
pub mod transaction_signer;
pub mod wallet;

pub use core::BitcoinCore;
pub use hardware_signer::{HardwareWalletManager, HardwareDevice, HardwareSigningRequest, HardwareSigningResponse};
pub use key_manager::{KeyManager, HDWallet, KeyDerivation, MnemonicInfo};
pub use psbt::{PsbtBuilder, PsbtSigner, AdvancedPsbtBuilder, PsbtWorkflowManager, MultiSigPsbtManager};
pub use psbt_advanced::{PsbtCombiner, PsbtFinalizer, PsbtUtils, HardwareWallet, MockHardwareWallet};
pub use rpc::{BitcoinRpc, RpcClient};
pub use script_types::{ScriptBuilder, ScriptTemplate, ScriptType, MultisigConfig};
pub use security_validator::{SecurityValidator, SecurityConfig, ValidationResult, SecurityLevel};
pub use transaction::{BitcoinTransaction, TransactionBuilder};
pub use transaction_signer::{TransactionSigner, SigningContext, InputSigningInfo, TransactionSigningResult};
pub use wallet::{BitcoinWallet, WalletManager};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Bitcoin network types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    /// Bitcoin mainnet
    Mainnet,
    /// Bitcoin testnet
    Testnet,
    /// Bitcoin regtest (local development)
    Regtest,
    /// Bitcoin signet
    Signet,
}

impl Network {
    /// Returns the network string for Bitcoin Core RPC
    pub fn as_str(&self) -> &'static str {
        match self {
            Network::Mainnet => "main",
            Network::Testnet => "test",
            Network::Regtest => "regtest",
            Network::Signet => "signet",
        }
    }

    /// Returns the address prefix for the network
    pub fn address_prefix(&self) -> &'static str {
        match self {
            Network::Mainnet => "bc1",
            Network::Testnet => "tb1",
            Network::Regtest => "bcrt1",
            Network::Signet => "tb1",
        }
    }
}

impl From<Network> for bitcoin::Network {
    fn from(network: Network) -> Self {
        match network {
            Network::Mainnet => bitcoin::Network::Bitcoin,
            Network::Testnet => bitcoin::Network::Testnet,
            Network::Regtest => bitcoin::Network::Regtest,
            Network::Signet => bitcoin::Network::Signet,
        }
    }
}

impl From<bitcoin::Network> for Network {
    fn from(network: bitcoin::Network) -> Self {
        match network {
            bitcoin::Network::Bitcoin => Network::Mainnet,
            bitcoin::Network::Testnet => Network::Testnet,
            bitcoin::Network::Regtest => Network::Regtest,
            bitcoin::Network::Signet => Network::Signet,
            _ => Network::Mainnet, // Default fallback
        }
    }
}

/// Bitcoin Core configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitcoinConfig {
    /// Bitcoin Core RPC URL
    pub rpc_url: String,
    /// RPC username
    pub rpc_user: String,
    /// RPC password
    pub rpc_password: String,
    /// Network type
    pub network: Network,
    /// Wallet name (optional)
    pub wallet_name: Option<String>,
    /// Connection timeout in seconds
    pub timeout: u64,
    /// Maximum number of retries
    pub max_retries: u32,
    /// Enable ZMQ notifications
    pub enable_zmq: bool,
    /// ZMQ endpoint for block notifications
    pub zmq_block_endpoint: Option<String>,
    /// ZMQ endpoint for transaction notifications
    pub zmq_tx_endpoint: Option<String>,
}

impl Default for BitcoinConfig {
    fn default() -> Self {
        Self {
            rpc_url: "http://127.0.0.1:8332".to_string(),
            rpc_user: "bitcoin".to_string(),
            rpc_password: "password".to_string(),
            network: Network::Regtest,
            wallet_name: Some("cerberus".to_string()),
            timeout: 30,
            max_retries: 3,
            enable_zmq: false,
            zmq_block_endpoint: None,
            zmq_tx_endpoint: None,
        }
    }
}

/// Bitcoin address types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AddressType {
    /// Legacy P2PKH addresses (1...)
    Legacy,
    /// P2SH-wrapped SegWit addresses (3...)
    P2shSegwit,
    /// Native SegWit addresses (bc1...)
    Bech32,
    /// Taproot addresses (bc1p...)
    Taproot,
}

impl AddressType {
    /// Returns the address type string for Bitcoin Core RPC
    pub fn as_str(&self) -> &'static str {
        match self {
            AddressType::Legacy => "legacy",
            AddressType::P2shSegwit => "p2sh-segwit",
            AddressType::Bech32 => "bech32",
            AddressType::Taproot => "bech32m",
        }
    }
}

/// Bitcoin amount in satoshis
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Amount(pub u64);

impl Amount {
    /// Create amount from BTC
    pub fn from_btc(btc: f64) -> Self {
        Self((btc * 100_000_000.0) as u64)
    }

    /// Create amount from satoshis
    pub fn from_sat(sat: u64) -> Self {
        Self(sat)
    }

    /// Convert to BTC
    pub fn to_btc(&self) -> f64 {
        self.0 as f64 / 100_000_000.0
    }

    /// Convert to satoshis
    pub fn to_sat(&self) -> u64 {
        self.0
    }

    /// Check if amount is zero
    pub fn is_zero(&self) -> bool {
        self.0 == 0
    }
}

impl std::fmt::Display for Amount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Format with exactly 8 decimal places, removing trailing zeros
        let btc_value = self.to_btc();
        let formatted = format!("{:.8}", btc_value);
        let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');
        write!(f, "{} BTC", trimmed)
    }
}

/// Bitcoin transaction output (UTXO)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Utxo {
    /// Transaction ID
    pub txid: String,
    /// Output index
    pub vout: u32,
    /// Amount in satoshis
    pub amount: Amount,
    /// Script public key
    pub script_pubkey: String,
    /// Address (if available)
    pub address: Option<String>,
    /// Number of confirmations
    pub confirmations: u32,
    /// Whether this UTXO is spendable
    pub spendable: bool,
    /// Whether this UTXO is safe to spend
    pub safe: bool,
}

/// Bitcoin transaction input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxInput {
    /// Previous transaction ID
    pub txid: String,
    /// Previous output index
    pub vout: u32,
    /// Script signature
    pub script_sig: Option<String>,
    /// Witness data
    pub witness: Option<Vec<String>>,
    /// Sequence number
    pub sequence: u32,
}

/// Bitcoin transaction output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxOutput {
    /// Amount in satoshis
    pub amount: Amount,
    /// Script public key
    pub script_pubkey: String,
    /// Address (if available)
    pub address: Option<String>,
}

/// Bitcoin fee estimation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeEstimate {
    /// Fee rate in sat/vB
    pub fee_rate: f64,
    /// Estimated fee for transaction
    pub estimated_fee: Amount,
    /// Target confirmation blocks
    pub target_blocks: u32,
}

/// Bitcoin Core connection status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStatus {
    /// Whether connected to Bitcoin Core
    pub connected: bool,
    /// Current block height
    pub block_height: u64,
    /// Network name
    pub network: String,
    /// Bitcoin Core version
    pub version: String,
    /// Number of connections
    pub connections: u32,
    /// Sync progress (0.0 - 1.0)
    pub sync_progress: f64,
    /// Whether initial block download is complete
    pub initial_block_download: bool,
}

/// Error types for Bitcoin operations
#[derive(Debug, thiserror::Error)]
pub enum BitcoinError {
    #[error("RPC error: {0}")]
    Rpc(String),
    
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Invalid address: {0}")]
    InvalidAddress(String),
    
    #[error("Insufficient funds: required {required}, available {available}")]
    InsufficientFunds { required: Amount, available: Amount },
    
    #[error("Transaction not found: {0}")]
    TransactionNotFound(String),
    
    #[error("Wallet not found: {0}")]
    WalletNotFound(String),
    
    #[error("Invalid PSBT: {0}")]
    InvalidPsbt(String),
    
    #[error("Signing error: {0}")]
    SigningError(String),

    #[error("Key management error: {0}")]
    KeyManagement(String),

    #[error("Hardware wallet error: {0}")]
    HardwareWallet(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Security validation error: {0}")]
    SecurityValidation(String),
}

pub type BitcoinResult<T> = Result<T, BitcoinError>;
