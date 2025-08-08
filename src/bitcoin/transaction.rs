//! Bitcoin transaction handling and building

use super::{Amount, BitcoinError, BitcoinResult, FeeEstimate, Utxo};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

/// Bitcoin transaction representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitcoinTransaction {
    /// Transaction ID
    pub txid: String,
    /// Transaction version
    pub version: u32,
    /// Lock time
    pub lock_time: u32,
    /// Transaction inputs
    pub inputs: Vec<TransactionInput>,
    /// Transaction outputs
    pub outputs: Vec<TransactionOutput>,
    /// Transaction size in bytes
    pub size: u32,
    /// Virtual size (for SegWit)
    pub vsize: u32,
    /// Weight (for SegWit)
    pub weight: u32,
    /// Transaction fee
    pub fee: Option<Amount>,
    /// Number of confirmations
    pub confirmations: u32,
    /// Block hash (if confirmed)
    pub block_hash: Option<String>,
    /// Block height (if confirmed)
    pub block_height: Option<u64>,
    /// Block time (if confirmed)
    pub block_time: Option<u64>,
}

/// Transaction input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionInput {
    /// Previous transaction ID
    pub prev_txid: String,
    /// Previous output index
    pub prev_vout: u32,
    /// Script signature
    pub script_sig: String,
    /// Witness data (for SegWit)
    pub witness: Vec<String>,
    /// Sequence number
    pub sequence: u32,
    /// Previous output amount (if known)
    pub prev_amount: Option<Amount>,
    /// Previous output address (if known)
    pub prev_address: Option<String>,
}

/// Transaction output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionOutput {
    /// Output amount
    pub amount: Amount,
    /// Script public key
    pub script_pubkey: String,
    /// Output address (if decodable)
    pub address: Option<String>,
    /// Output index
    pub n: u32,
}

/// Transaction builder for creating Bitcoin transactions
pub struct TransactionBuilder {
    inputs: Vec<TransactionInput>,
    outputs: Vec<TransactionOutput>,
    lock_time: u32,
    version: u32,
}

impl TransactionBuilder {
    /// Create new transaction builder
    pub fn new() -> Self {
        Self {
            inputs: Vec::new(),
            outputs: Vec::new(),
            lock_time: 0,
            version: 2, // BIP 68 compatible
        }
    }

    /// Set transaction version
    pub fn version(mut self, version: u32) -> Self {
        self.version = version;
        self
    }

    /// Set lock time
    pub fn lock_time(mut self, lock_time: u32) -> Self {
        self.lock_time = lock_time;
        self
    }

    /// Add input from UTXO
    pub fn add_input_from_utxo(mut self, utxo: &Utxo) -> Self {
        let input = TransactionInput {
            prev_txid: utxo.txid.clone(),
            prev_vout: utxo.vout,
            script_sig: String::new(), // Will be filled during signing
            witness: Vec::new(),
            sequence: 0xfffffffe, // RBF enabled
            prev_amount: Some(utxo.amount),
            prev_address: utxo.address.clone(),
        };
        self.inputs.push(input);
        self
    }

    /// Add output
    pub fn add_output(mut self, address: String, amount: Amount) -> Self {
        let output = TransactionOutput {
            amount,
            script_pubkey: self.address_to_script_pubkey(&address),
            address: Some(address),
            n: self.outputs.len() as u32,
        };
        self.outputs.push(output);
        self
    }

    /// Build transaction with automatic UTXO selection and change handling
    pub async fn build_transaction(
        &self,
        available_utxos: Vec<Utxo>,
        target_outputs: HashMap<String, Amount>,
        fee_estimate: FeeEstimate,
        change_address: Option<String>,
    ) -> BitcoinResult<(String, Amount)> {
        // Calculate total output amount
        let total_output: u64 = target_outputs.values().map(|a| a.to_sat()).sum();
        let total_output_amount = Amount::from_sat(total_output);

        // Estimate transaction size and fee
        let estimated_size = self.estimate_transaction_size(target_outputs.len(), change_address.is_some());
        let estimated_fee = Amount::from_sat((fee_estimate.fee_rate * estimated_size as f64) as u64);

        // Total amount needed (outputs + fee)
        let total_needed = Amount::from_sat(total_output_amount.to_sat() + estimated_fee.to_sat());

        // Select UTXOs
        let selected_utxos = self.select_utxos(&available_utxos, total_needed)?;
        
        // Calculate total input amount
        let total_input: u64 = selected_utxos.iter().map(|u| u.amount.to_sat()).sum();
        let total_input_amount = Amount::from_sat(total_input);

        // Calculate change
        let change_amount = Amount::from_sat(
            total_input_amount.to_sat()
                .saturating_sub(total_output_amount.to_sat())
                .saturating_sub(estimated_fee.to_sat())
        );

        // Build transaction
        let mut builder = TransactionBuilder::new();

        // Add inputs
        for utxo in &selected_utxos {
            builder = builder.add_input_from_utxo(utxo);
        }

        // Add outputs
        for (address, amount) in target_outputs {
            builder = builder.add_output(address, amount);
        }

        // Add change output if needed
        if change_amount.to_sat() > 546 { // Dust threshold
            if let Some(change_addr) = change_address {
                builder = builder.add_output(change_addr, change_amount);
            } else {
                return Err(BitcoinError::Rpc("Change address required but not provided".to_string()));
            }
        }

        // Create raw transaction hex (simplified)
        let raw_tx = builder.to_raw_transaction()?;
        
        debug!("Built transaction with {} inputs, {} outputs, fee: {}", 
               selected_utxos.len(), builder.outputs.len(), estimated_fee);

        Ok((raw_tx, estimated_fee))
    }

    /// Select UTXOs for transaction (simple greedy algorithm)
    fn select_utxos(&self, available_utxos: &[Utxo], target_amount: Amount) -> BitcoinResult<Vec<Utxo>> {
        let mut selected = Vec::new();
        let mut total_selected = 0u64;

        // Sort UTXOs by amount (largest first)
        let mut sorted_utxos = available_utxos.to_vec();
        sorted_utxos.sort_by(|a, b| b.amount.to_sat().cmp(&a.amount.to_sat()));

        for utxo in sorted_utxos {
            if !utxo.spendable || !utxo.safe {
                continue;
            }

            total_selected += utxo.amount.to_sat();
            selected.push(utxo);

            if total_selected >= target_amount.to_sat() {
                break;
            }
        }

        if total_selected < target_amount.to_sat() {
            return Err(BitcoinError::InsufficientFunds {
                required: target_amount,
                available: Amount::from_sat(total_selected),
            });
        }

        Ok(selected)
    }

    /// Estimate transaction size in vBytes
    pub fn estimate_transaction_size(&self, num_outputs: usize, has_change: bool) -> u32 {
        // Simplified estimation
        // Real implementation would consider input types (P2PKH, P2WPKH, etc.)
        let num_inputs = self.inputs.len();
        let total_outputs = num_outputs + if has_change { 1 } else { 0 };

        // Base transaction size
        let base_size = 10; // version (4) + input count (1) + output count (1) + lock time (4)
        
        // Input size (varies by type, using P2WPKH estimate)
        let input_size = num_inputs * 68; // 32 (txid) + 4 (vout) + 1 (script len) + 25 (script) + 4 (sequence) + 2 (witness)
        
        // Output size
        let output_size = total_outputs * 34; // 8 (amount) + 1 (script len) + 25 (script)

        (base_size + input_size + output_size) as u32
    }

    /// Convert address to script public key (simplified)
    fn address_to_script_pubkey(&self, address: &str) -> String {
        // This is a simplified implementation
        // Real implementation would properly decode address and create script
        if address.starts_with("bc1") || address.starts_with("tb1") {
            // Bech32 (SegWit)
            format!("0014{}", address) // Simplified
        } else if address.starts_with("3") || address.starts_with("2") {
            // P2SH
            format!("a914{}87", address) // Simplified
        } else {
            // P2PKH
            format!("76a914{}88ac", address) // Simplified
        }
    }

    /// Convert to raw transaction hex
    fn to_raw_transaction(&self) -> BitcoinResult<String> {
        // This is a simplified implementation
        // Real implementation would properly serialize according to Bitcoin protocol
        
        let mut tx_hex = String::new();
        
        // Version (4 bytes, little endian)
        tx_hex.push_str(&format!("{:08x}", self.version.swap_bytes()));
        
        // Input count (varint)
        tx_hex.push_str(&format!("{:02x}", self.inputs.len()));
        
        // Inputs
        for input in &self.inputs {
            // Previous output hash (32 bytes, reversed)
            tx_hex.push_str(&input.prev_txid);
            // Previous output index (4 bytes, little endian)
            tx_hex.push_str(&format!("{:08x}", input.prev_vout.swap_bytes()));
            // Script length (varint) - 0 for unsigned
            tx_hex.push_str("00");
            // Sequence (4 bytes, little endian)
            tx_hex.push_str(&format!("{:08x}", input.sequence.swap_bytes()));
        }
        
        // Output count (varint)
        tx_hex.push_str(&format!("{:02x}", self.outputs.len()));
        
        // Outputs
        for output in &self.outputs {
            // Amount (8 bytes, little endian)
            tx_hex.push_str(&format!("{:016x}", output.amount.to_sat().swap_bytes()));
            // Script length (varint)
            let script_len = output.script_pubkey.len() / 2;
            tx_hex.push_str(&format!("{:02x}", script_len));
            // Script
            tx_hex.push_str(&output.script_pubkey);
        }
        
        // Lock time (4 bytes, little endian)
        tx_hex.push_str(&format!("{:08x}", self.lock_time.swap_bytes()));
        
        Ok(tx_hex)
    }

    /// Get total input amount
    pub fn total_input_amount(&self) -> Amount {
        let total: u64 = self.inputs.iter()
            .filter_map(|i| i.prev_amount)
            .map(|a| a.to_sat())
            .sum();
        Amount::from_sat(total)
    }

    /// Get total output amount
    pub fn total_output_amount(&self) -> Amount {
        let total: u64 = self.outputs.iter().map(|o| o.amount.to_sat()).sum();
        Amount::from_sat(total)
    }

    /// Calculate transaction fee
    pub fn calculate_fee(&self) -> Amount {
        let input_total = self.total_input_amount();
        let output_total = self.total_output_amount();
        Amount::from_sat(input_total.to_sat().saturating_sub(output_total.to_sat()))
    }
}

impl Default for TransactionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl BitcoinTransaction {
    /// Create transaction from raw hex
    pub fn from_hex(hex: &str) -> BitcoinResult<Self> {
        // This is a placeholder implementation
        // Real implementation would parse the raw transaction hex
        Err(BitcoinError::Rpc("Transaction parsing from hex not implemented".to_string()))
    }

    /// Convert transaction to hex
    pub fn to_hex(&self) -> String {
        // This is a placeholder implementation
        // Real implementation would serialize the transaction to hex
        format!("placeholder_hex_{}", self.txid)
    }

    /// Check if transaction is confirmed
    pub fn is_confirmed(&self) -> bool {
        self.confirmations > 0
    }

    /// Get transaction fee rate in sat/vB
    pub fn fee_rate(&self) -> Option<f64> {
        self.fee.map(|fee| fee.to_sat() as f64 / self.vsize as f64)
    }

    /// Check if transaction is RBF (Replace-By-Fee) enabled
    pub fn is_rbf_enabled(&self) -> bool {
        self.inputs.iter().any(|input| input.sequence < 0xfffffffe)
    }
}
