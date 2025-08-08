//! Bitcoin Core integration main module

use super::{
    rpc::{BitcoinRpc, RpcClient},
    BitcoinConfig, BitcoinError, BitcoinResult, Amount, ConnectionStatus, FeeEstimate,
    Network, Utxo, AddressType,
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

/// Main Bitcoin Core integration client
pub struct BitcoinCore {
    rpc: RpcClient,
    config: BitcoinConfig,
}

impl BitcoinCore {
    /// Create new Bitcoin Core client
    pub async fn new(config: BitcoinConfig) -> Result<Self> {
        let rpc = RpcClient::new(config.clone())
            .context("Failed to create Bitcoin RPC client")?;

        let core = Self { rpc, config };

        // Test connection
        core.test_connection().await
            .context("Failed to connect to Bitcoin Core")?;

        info!("Bitcoin Core client initialized successfully");
        Ok(core)
    }

    /// Test connection to Bitcoin Core
    pub async fn test_connection(&self) -> BitcoinResult<ConnectionStatus> {
        self.rpc.test_connection().await
    }

    /// Get current block height
    pub async fn get_block_height(&self) -> BitcoinResult<u64> {
        let info = self.rpc.get_blockchain_info().await?;
        Ok(info.blocks)
    }

    /// Get network type
    pub async fn get_network(&self) -> BitcoinResult<Network> {
        let info = self.rpc.get_blockchain_info().await?;
        match info.chain.as_str() {
            "main" => Ok(Network::Mainnet),
            "test" => Ok(Network::Testnet),
            "regtest" => Ok(Network::Regtest),
            "signet" => Ok(Network::Signet),
            _ => Err(BitcoinError::Rpc(format!("Unknown network: {}", info.chain))),
        }
    }

    /// Generate new address
    pub async fn generate_address(&self, address_type: AddressType, label: Option<&str>) -> BitcoinResult<String> {
        self.rpc.get_new_address(label, Some(address_type.as_str())).await
    }

    /// Get wallet balance
    pub async fn get_balance(&self) -> BitcoinResult<Amount> {
        let balance_btc = self.rpc.get_balance().await?;
        Ok(Amount::from_btc(balance_btc))
    }

    /// Get unconfirmed balance
    pub async fn get_unconfirmed_balance(&self) -> BitcoinResult<Amount> {
        let balance_btc = self.rpc.get_unconfirmed_balance().await?;
        Ok(Amount::from_btc(balance_btc))
    }

    /// Get total balance (confirmed + unconfirmed)
    pub async fn get_total_balance(&self) -> BitcoinResult<Amount> {
        let confirmed = self.get_balance().await?;
        let unconfirmed = self.get_unconfirmed_balance().await?;
        Ok(Amount::from_sat(confirmed.to_sat() + unconfirmed.to_sat()))
    }

    /// List unspent outputs (UTXOs)
    pub async fn list_utxos(&self, min_confirmations: Option<u32>, addresses: Option<Vec<String>>) -> BitcoinResult<Vec<BitcoinUtxo>> {
        let utxos = self.rpc.list_unspent(min_confirmations, None, addresses).await?;
        
        Ok(utxos.into_iter().map(|utxo| BitcoinUtxo {
            txid: utxo.txid,
            vout: utxo.vout,
            amount: Amount::from_btc(utxo.amount),
            address: utxo.address,
            script_pubkey: utxo.script_pubkey,
            confirmations: utxo.confirmations,
            spendable: utxo.spendable,
            safe: utxo.safe,
            label: utxo.label,
        }).collect())
    }

    /// Send Bitcoin to address
    pub async fn send_to_address(&self, address: &str, amount: Amount, comment: Option<&str>) -> BitcoinResult<String> {
        // Validate address first
        self.validate_address(address).await?;
        
        let amount_btc = amount.to_btc();
        self.rpc.send_to_address(address, amount_btc, comment).await
    }

    /// Create and send transaction with multiple outputs
    pub async fn send_many(&self, outputs: HashMap<String, Amount>) -> BitcoinResult<String> {
        // Validate all addresses
        for address in outputs.keys() {
            self.validate_address(address).await?;
        }

        // Calculate total amount needed
        let total_amount: u64 = outputs.values().map(|a| a.to_sat()).sum();
        let total_amount = Amount::from_sat(total_amount);

        // Check if we have enough balance
        let available_balance = self.get_balance().await?;
        if available_balance.to_sat() < total_amount.to_sat() {
            return Err(BitcoinError::InsufficientFunds {
                required: total_amount,
                available: available_balance,
            });
        }

        // Get UTXOs for transaction
        let utxos = self.list_utxos(Some(1), None).await?;
        
        // Build transaction
        let tx_builder = TransactionBuilder::new();
        let (raw_tx, _) = tx_builder.build_transaction(utxos, outputs, self.estimate_fee(6).await?).await?;

        // Sign and send
        let signed_tx = self.rpc.sign_raw_transaction_with_wallet(&raw_tx).await?;
        
        if !signed_tx.complete {
            return Err(BitcoinError::SigningError("Transaction signing incomplete".to_string()));
        }

        self.rpc.send_raw_transaction(&signed_tx.hex).await
    }

    /// Estimate transaction fee
    pub async fn estimate_fee(&self, target_blocks: u32) -> BitcoinResult<FeeEstimate> {
        let fee_result = self.rpc.estimate_smart_fee(target_blocks).await?;
        
        let fee_rate = fee_result.feerate.unwrap_or(0.00001); // Default to 1 sat/vB if no estimate
        
        // Estimate transaction size (rough calculation)
        let estimated_size = 250; // Average transaction size in vBytes
        let estimated_fee = Amount::from_btc(fee_rate * estimated_size as f64 / 100_000_000.0);

        Ok(FeeEstimate {
            fee_rate,
            estimated_fee,
            target_blocks,
        })
    }

    /// Validate Bitcoin address
    pub async fn validate_address(&self, address: &str) -> BitcoinResult<bool> {
        // Basic validation - check if address starts with correct prefix for network
        let network = self.get_network().await?;
        let valid_prefixes = match network {
            Network::Mainnet => vec!["1", "3", "bc1"],
            Network::Testnet => vec!["m", "n", "2", "tb1"],
            Network::Regtest => vec!["m", "n", "2", "bcrt1"],
            Network::Signet => vec!["tb1"],
        };

        let is_valid = valid_prefixes.iter().any(|prefix| address.starts_with(prefix));
        
        if !is_valid {
            return Err(BitcoinError::InvalidAddress(format!(
                "Address {} is not valid for network {:?}",
                address, network
            )));
        }

        Ok(true)
    }

    /// Get transaction details
    pub async fn get_transaction(&self, txid: &str) -> BitcoinResult<BitcoinTransactionInfo> {
        let tx_info = self.rpc.get_transaction(txid).await?;
        
        Ok(BitcoinTransactionInfo {
            txid: tx_info.txid,
            amount: Amount::from_btc(tx_info.amount),
            fee: tx_info.fee.map(|f| Amount::from_btc(f.abs())),
            confirmations: tx_info.confirmations,
            block_hash: tx_info.blockhash,
            block_time: tx_info.blocktime,
            time: tx_info.time,
            details: tx_info.details.into_iter().map(|d| TransactionDetail {
                address: d.address,
                category: d.category,
                amount: Amount::from_btc(d.amount),
                label: d.label,
            }).collect(),
        })
    }

    /// Create wallet if it doesn't exist
    pub async fn create_wallet(&self, wallet_name: &str, disable_private_keys: bool) -> BitcoinResult<()> {
        // This would require additional RPC calls like createwallet
        // For now, we assume wallet exists or is created externally
        info!("Wallet operations require manual setup in Bitcoin Core");
        Ok(())
    }

    /// Load wallet
    pub async fn load_wallet(&self, wallet_name: &str) -> BitcoinResult<()> {
        // This would require loadwallet RPC call
        info!("Loading wallet: {}", wallet_name);
        Ok(())
    }

    /// Get wallet info
    pub async fn get_wallet_info(&self) -> BitcoinResult<BitcoinWalletInfo> {
        let wallet_info = self.rpc.get_wallet_info().await?;
        
        Ok(BitcoinWalletInfo {
            name: wallet_info.walletname,
            version: wallet_info.walletversion,
            balance: Amount::from_btc(wallet_info.balance),
            unconfirmed_balance: Amount::from_btc(wallet_info.unconfirmed_balance),
            immature_balance: Amount::from_btc(wallet_info.immature_balance),
            transaction_count: wallet_info.txcount,
            keypool_size: wallet_info.keypoolsize,
            private_keys_enabled: wallet_info.private_keys_enabled,
        })
    }

    /// Health check for Bitcoin Core connection
    pub async fn health_check(&self) -> BitcoinResult<HealthStatus> {
        match self.test_connection().await {
            Ok(status) => Ok(HealthStatus {
                connected: status.connected,
                block_height: status.block_height,
                network: status.network,
                sync_progress: status.sync_progress,
                error: None,
            }),
            Err(e) => Ok(HealthStatus {
                connected: false,
                block_height: 0,
                network: "unknown".to_string(),
                sync_progress: 0.0,
                error: Some(e.to_string()),
            }),
        }
    }
}

/// Bitcoin UTXO representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitcoinUtxo {
    pub txid: String,
    pub vout: u32,
    pub amount: Amount,
    pub address: String,
    pub script_pubkey: String,
    pub confirmations: u32,
    pub spendable: bool,
    pub safe: bool,
    pub label: Option<String>,
}

/// Bitcoin transaction information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitcoinTransactionInfo {
    pub txid: String,
    pub amount: Amount,
    pub fee: Option<Amount>,
    pub confirmations: i32,
    pub block_hash: Option<String>,
    pub block_time: Option<u64>,
    pub time: u64,
    pub details: Vec<TransactionDetail>,
}

/// Transaction detail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionDetail {
    pub address: String,
    pub category: String,
    pub amount: Amount,
    pub label: Option<String>,
}

/// Bitcoin wallet information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitcoinWalletInfo {
    pub name: String,
    pub version: u32,
    pub balance: Amount,
    pub unconfirmed_balance: Amount,
    pub immature_balance: Amount,
    pub transaction_count: u32,
    pub keypool_size: u32,
    pub private_keys_enabled: bool,
}

/// Health status for Bitcoin Core
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub connected: bool,
    pub block_height: u64,
    pub network: String,
    pub sync_progress: f64,
    pub error: Option<String>,
}

/// Transaction builder for creating raw transactions
pub struct TransactionBuilder;

impl TransactionBuilder {
    pub fn new() -> Self {
        Self
    }

    pub async fn build_transaction(
        &self,
        utxos: Vec<BitcoinUtxo>,
        outputs: HashMap<String, Amount>,
        fee_estimate: FeeEstimate,
    ) -> BitcoinResult<(String, Amount)> {
        // This is a simplified implementation
        // In a real implementation, you would:
        // 1. Select appropriate UTXOs
        // 2. Calculate exact fees
        // 3. Handle change outputs
        // 4. Build the raw transaction hex
        
        // For now, return a placeholder
        Err(BitcoinError::Rpc("Transaction building not fully implemented".to_string()))
    }
}
