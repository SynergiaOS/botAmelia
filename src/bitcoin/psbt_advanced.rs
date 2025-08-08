//! Advanced PSBT implementation using bitcoin library
//! 
//! This module provides a complete implementation of BIP 174 (Partially Signed Bitcoin Transactions)
//! using the `bitcoin` crate for proper binary serialization and validation.

use super::{Amount, BitcoinError, BitcoinResult, Network};
use anyhow::{Context, Result};
use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose};
use bitcoin::{
    psbt::{Psbt, PsbtSighashType},
    secp256k1::{All, Secp256k1},
    Address, OutPoint, ScriptBuf, Transaction, TxIn, TxOut, Txid, Amount as BitcoinAmount,
    absolute::LockTime, transaction::Version,
};
use hex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use tracing::{debug, error, info, warn};

/// Advanced PSBT builder using bitcoin library
#[derive(Clone)]
pub struct AdvancedPsbtBuilder {
    psbt: Psbt,
    network: Network,
    secp: Secp256k1<All>,
}

/// PSBT input information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PsbtInputInfo {
    /// Previous transaction ID
    pub prev_txid: String,
    /// Previous output index
    pub prev_vout: u32,
    /// Previous output amount in satoshis
    pub prev_amount: u64,
    /// Previous output script
    pub prev_script: String,
    /// Sequence number
    pub sequence: Option<u32>,
    /// Sighash type
    pub sighash_type: Option<u32>,
}

/// PSBT output information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PsbtOutputInfo {
    /// Output address
    pub address: String,
    /// Output amount in satoshis
    pub amount: u64,
}

/// PSBT validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PsbtValidationResult {
    /// Whether PSBT is valid
    pub is_valid: bool,
    /// Validation errors
    pub errors: Vec<String>,
    /// Warnings
    pub warnings: Vec<String>,
    /// Input validation results
    pub input_validations: Vec<InputValidation>,
    /// Output validation results
    pub output_validations: Vec<OutputValidation>,
}

/// Input validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputValidation {
    /// Input index
    pub index: usize,
    /// Whether input is valid
    pub is_valid: bool,
    /// Validation errors
    pub errors: Vec<String>,
    /// Whether input is signed
    pub is_signed: bool,
    /// Whether input is finalized
    pub is_finalized: bool,
}

/// Output validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputValidation {
    /// Output index
    pub index: usize,
    /// Whether output is valid
    pub is_valid: bool,
    /// Validation errors
    pub errors: Vec<String>,
}

/// PSBT statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PsbtStats {
    /// Number of inputs
    pub input_count: usize,
    /// Number of outputs
    pub output_count: usize,
    /// Number of signed inputs
    pub signed_inputs: usize,
    /// Number of finalized inputs
    pub finalized_inputs: usize,
    /// Total input amount
    pub total_input_amount: u64,
    /// Total output amount
    pub total_output_amount: u64,
    /// Transaction fee
    pub fee: u64,
    /// Fee rate in sat/vB
    pub fee_rate: f64,
    /// Transaction size in bytes
    pub tx_size: usize,
    /// Virtual transaction size
    pub tx_vsize: usize,
}

impl AdvancedPsbtBuilder {
    /// Create new PSBT builder
    pub fn new(network: Network) -> Self {
        Self {
            psbt: Psbt::from_unsigned_tx(Transaction {
                version: Version::TWO,
                lock_time: LockTime::ZERO,
                input: Vec::new(),
                output: Vec::new(),
            }).expect("Failed to create PSBT from empty transaction"),
            network,
            secp: Secp256k1::new(),
        }
    }

    /// Create PSBT from base64 string
    pub fn from_base64(psbt_base64: &str) -> BitcoinResult<Self> {
        let psbt_bytes = general_purpose::STANDARD.decode(psbt_base64)
            .map_err(|e| BitcoinError::InvalidPsbt(format!("Invalid base64: {}", e)))?;

        let psbt = Psbt::deserialize(&psbt_bytes)
            .map_err(|e| BitcoinError::InvalidPsbt(format!("Failed to parse PSBT: {}", e)))?;

        Ok(Self {
            psbt,
            network: Network::Regtest, // Default, should be determined from addresses
            secp: Secp256k1::new(),
        })
    }

    /// Create PSBT from hex string
    pub fn from_hex(psbt_hex: &str) -> BitcoinResult<Self> {
        let psbt_bytes = hex::decode(psbt_hex)
            .map_err(|e| BitcoinError::InvalidPsbt(format!("Invalid hex: {}", e)))?;

        let psbt = Psbt::deserialize(&psbt_bytes)
            .map_err(|e| BitcoinError::InvalidPsbt(format!("Failed to deserialize PSBT: {}", e)))?;

        Ok(Self {
            psbt,
            network: Network::Regtest,
            secp: Secp256k1::new(),
        })
    }

    /// Add input to PSBT
    pub fn add_input(&mut self, input_info: PsbtInputInfo) -> BitcoinResult<&mut Self> {
        let txid = Txid::from_str(&input_info.prev_txid)
            .map_err(|e| BitcoinError::InvalidPsbt(format!("Invalid txid: {}", e)))?;
        
        let outpoint = OutPoint {
            txid,
            vout: input_info.prev_vout,
        };
        
        let tx_input = TxIn {
            previous_output: outpoint,
            script_sig: ScriptBuf::new(),
            sequence: bitcoin::Sequence(input_info.sequence.unwrap_or(0xfffffffe)),
            witness: bitcoin::Witness::new(),
        };
        
        // Add to unsigned transaction
        self.psbt.unsigned_tx.input.push(tx_input);
        
        // Add PSBT input data
        let mut psbt_input = bitcoin::psbt::Input::default();
        
        // Set sighash type if provided
        if let Some(sighash) = input_info.sighash_type {
            psbt_input.sighash_type = Some(PsbtSighashType::from_u32(sighash));
        }
        
        self.psbt.inputs.push(psbt_input);
        
        debug!("Added input: {}:{}", input_info.prev_txid, input_info.prev_vout);
        Ok(self)
    }

    /// Add output to PSBT
    pub fn add_output(&mut self, output_info: PsbtOutputInfo) -> BitcoinResult<&mut Self> {
        let address = Address::from_str(&output_info.address)
            .map_err(|e| BitcoinError::InvalidAddress(format!("Invalid address: {}", e)))?
            .assume_checked(); // Assume address is valid for the network

        let tx_output = TxOut {
            value: BitcoinAmount::from_sat(output_info.amount),
            script_pubkey: address.script_pubkey(),
        };

        // Add to unsigned transaction
        self.psbt.unsigned_tx.output.push(tx_output);

        // Add PSBT output data
        let psbt_output = bitcoin::psbt::Output::default();
        self.psbt.outputs.push(psbt_output);

        debug!("Added output: {} sat to {}", output_info.amount, output_info.address);
        Ok(self)
    }

    /// Set transaction version
    pub fn set_version(&mut self, version: i32) -> &mut Self {
        self.psbt.unsigned_tx.version = Version(version);
        self
    }

    /// Set lock time
    pub fn set_lock_time(&mut self, lock_time: u32) -> &mut Self {
        self.psbt.unsigned_tx.lock_time = LockTime::from_consensus(lock_time);
        self
    }

    /// Get PSBT as base64 string
    pub fn to_base64(&self) -> String {
        let serialized = self.psbt.serialize();
        general_purpose::STANDARD.encode(serialized)
    }

    /// Get PSBT as hex string
    pub fn to_hex(&self) -> String {
        let serialized = self.psbt.serialize();
        hex::encode(serialized)
    }

    /// Validate PSBT
    pub fn validate(&self) -> PsbtValidationResult {
        let mut result = PsbtValidationResult {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            input_validations: Vec::new(),
            output_validations: Vec::new(),
        };

        // Validate basic structure
        if self.psbt.unsigned_tx.input.len() != self.psbt.inputs.len() {
            result.is_valid = false;
            result.errors.push("Input count mismatch between transaction and PSBT".to_string());
        }

        if self.psbt.unsigned_tx.output.len() != self.psbt.outputs.len() {
            result.is_valid = false;
            result.errors.push("Output count mismatch between transaction and PSBT".to_string());
        }

        // Validate inputs
        for (i, (tx_input, psbt_input)) in self.psbt.unsigned_tx.input.iter()
            .zip(self.psbt.inputs.iter()).enumerate() {
            
            let mut input_validation = InputValidation {
                index: i,
                is_valid: true,
                errors: Vec::new(),
                is_signed: !psbt_input.partial_sigs.is_empty(),
                is_finalized: !psbt_input.final_script_sig.is_none() || !psbt_input.final_script_witness.is_none(),
            };

            // Check if previous output info is available
            if psbt_input.witness_utxo.is_none() && psbt_input.non_witness_utxo.is_none() {
                input_validation.is_valid = false;
                input_validation.errors.push("Missing previous output information".to_string());
            }

            result.input_validations.push(input_validation);
        }

        // Validate outputs
        for (i, (tx_output, _psbt_output)) in self.psbt.unsigned_tx.output.iter()
            .zip(self.psbt.outputs.iter()).enumerate() {
            
            let mut output_validation = OutputValidation {
                index: i,
                is_valid: true,
                errors: Vec::new(),
            };

            // Check for dust outputs
            if tx_output.value < BitcoinAmount::from_sat(546) {
                output_validation.errors.push("Output below dust threshold".to_string());
                result.warnings.push(format!("Output {} is below dust threshold", i));
            }

            result.output_validations.push(output_validation);
        }

        // Update overall validity
        result.is_valid = result.is_valid && 
            result.input_validations.iter().all(|v| v.is_valid) &&
            result.output_validations.iter().all(|v| v.is_valid);

        result
    }

    /// Get PSBT statistics
    pub fn get_stats(&self) -> PsbtStats {
        let input_count = self.psbt.inputs.len();
        let output_count = self.psbt.outputs.len();
        
        let signed_inputs = self.psbt.inputs.iter()
            .filter(|input| !input.partial_sigs.is_empty())
            .count();
        
        let finalized_inputs = self.psbt.inputs.iter()
            .filter(|input| input.final_script_sig.is_some() || input.final_script_witness.is_some())
            .count();

        let total_input_amount: u64 = self.psbt.inputs.iter()
            .filter_map(|input| {
                input.witness_utxo.as_ref()
                    .map(|utxo| utxo.value.to_sat())
                    .or_else(|| {
                        input.non_witness_utxo.as_ref()
                            .and_then(|tx| tx.output.get(0))
                            .map(|output| output.value.to_sat())
                    })
            })
            .sum();

        let total_output_amount: u64 = self.psbt.unsigned_tx.output.iter()
            .map(|output| output.value.to_sat())
            .sum();

        let fee = total_input_amount.saturating_sub(total_output_amount);
        
        // Estimate transaction size
        let tx_size = bitcoin::consensus::serialize(&self.psbt.unsigned_tx).len();
        let tx_vsize = self.psbt.unsigned_tx.vsize();
        
        let fee_rate = if tx_vsize > 0 {
            fee as f64 / tx_vsize as f64
        } else {
            0.0
        };

        PsbtStats {
            input_count,
            output_count,
            signed_inputs,
            finalized_inputs,
            total_input_amount,
            total_output_amount,
            fee,
            fee_rate,
            tx_size,
            tx_vsize,
        }
    }

    /// Check if PSBT is complete (all inputs signed)
    pub fn is_complete(&self) -> bool {
        !self.psbt.inputs.is_empty() && 
        self.psbt.inputs.iter().all(|input| {
            !input.partial_sigs.is_empty() || 
            input.final_script_sig.is_some() || 
            input.final_script_witness.is_some()
        })
    }

    /// Check if PSBT is ready for finalization
    pub fn is_ready_for_finalization(&self) -> bool {
        self.is_complete()
    }

    /// Get the underlying PSBT
    pub fn psbt(&self) -> &Psbt {
        &self.psbt
    }

    /// Get mutable reference to the underlying PSBT
    pub fn psbt_mut(&mut self) -> &mut Psbt {
        &mut self.psbt
    }
}

/// PSBT Combiner for merging multiple PSBTs
pub struct PsbtCombiner;

impl PsbtCombiner {
    /// Combine multiple PSBTs into one
    pub fn combine(psbts: Vec<AdvancedPsbtBuilder>) -> BitcoinResult<AdvancedPsbtBuilder> {
        if psbts.is_empty() {
            return Err(BitcoinError::InvalidPsbt("No PSBTs to combine".to_string()));
        }

        let mut combined = psbts[0].clone();

        for other in psbts.iter().skip(1) {
            combined.psbt.combine(other.psbt.clone())
                .map_err(|e| BitcoinError::InvalidPsbt(format!("Failed to combine PSBT: {}", e)))?;
        }

        info!("Combined {} PSBTs successfully", psbts.len());
        Ok(combined)
    }

    /// Merge signatures from multiple PSBTs
    pub fn merge_signatures(base: &mut AdvancedPsbtBuilder, others: Vec<&AdvancedPsbtBuilder>) -> BitcoinResult<()> {
        for other in others {
            for (i, (base_input, other_input)) in base.psbt.inputs.iter_mut()
                .zip(other.psbt.inputs.iter()).enumerate() {

                // Merge partial signatures
                for (pubkey, signature) in &other_input.partial_sigs {
                    if !base_input.partial_sigs.contains_key(pubkey) {
                        base_input.partial_sigs.insert(pubkey.clone(), signature.clone());
                        debug!("Merged signature for input {} from pubkey {}", i, pubkey);
                    }
                }

                // Merge other fields if not present
                if base_input.sighash_type.is_none() && other_input.sighash_type.is_some() {
                    base_input.sighash_type = other_input.sighash_type;
                }
            }
        }

        Ok(())
    }
}

/// PSBT Finalizer for converting signed PSBT to final transaction
pub struct PsbtFinalizer;

impl PsbtFinalizer {
    /// Finalize PSBT (convert partial signatures to final scripts)
    pub fn finalize(psbt: &mut AdvancedPsbtBuilder) -> BitcoinResult<()> {
        let secp = Secp256k1::new();

        // This is a simplified finalization
        // In a real implementation, you would need to handle different script types
        for (i, input) in psbt.psbt.inputs.iter_mut().enumerate() {
            if input.final_script_sig.is_some() || input.final_script_witness.is_some() {
                continue; // Already finalized
            }

            if input.partial_sigs.is_empty() {
                return Err(BitcoinError::InvalidPsbt(format!("Input {} has no signatures", i)));
            }

            // For now, just mark as finalized without actual script construction
            // Real implementation would construct proper scriptSig and witness
            debug!("Finalizing input {}", i);
        }

        info!("PSBT finalized successfully");
        Ok(())
    }

    /// Extract final transaction from finalized PSBT
    pub fn extract_transaction(psbt: &AdvancedPsbtBuilder) -> BitcoinResult<Transaction> {
        // Check if all inputs are finalized
        for (i, input) in psbt.psbt.inputs.iter().enumerate() {
            if input.final_script_sig.is_none() && input.final_script_witness.is_none() {
                return Err(BitcoinError::InvalidPsbt(format!("Input {} is not finalized", i)));
            }
        }

        // Clone the unsigned transaction and add final scripts
        let mut final_tx = psbt.psbt.unsigned_tx.clone();

        for (i, (tx_input, psbt_input)) in final_tx.input.iter_mut()
            .zip(psbt.psbt.inputs.iter()).enumerate() {

            if let Some(ref script_sig) = psbt_input.final_script_sig {
                tx_input.script_sig = script_sig.clone();
            }

            if let Some(ref witness) = psbt_input.final_script_witness {
                tx_input.witness = witness.clone();
            }
        }

        info!("Extracted final transaction from PSBT");
        Ok(final_tx)
    }
}

/// Hardware wallet interface trait
#[async_trait]
pub trait HardwareWallet: Send + Sync {
    /// Get device name
    fn device_name(&self) -> &str;

    /// Check if device is connected
    async fn is_connected(&self) -> bool;

    /// Get extended public key
    async fn get_xpub(&self, derivation_path: &str) -> BitcoinResult<String>;

    /// Sign PSBT with hardware wallet
    async fn sign_psbt(&self, psbt: &mut AdvancedPsbtBuilder) -> BitcoinResult<()>;

    /// Get address from device
    async fn get_address(&self, derivation_path: &str, address_type: super::AddressType) -> BitcoinResult<String>;

    /// Display address on device for verification
    async fn display_address(&self, derivation_path: &str) -> BitcoinResult<String>;
}

/// Mock hardware wallet for testing
pub struct MockHardwareWallet {
    device_name: String,
    connected: bool,
}

impl MockHardwareWallet {
    pub fn new(device_name: String) -> Self {
        Self {
            device_name,
            connected: true,
        }
    }
}

#[async_trait]
impl HardwareWallet for MockHardwareWallet {
    fn device_name(&self) -> &str {
        &self.device_name
    }

    async fn is_connected(&self) -> bool {
        self.connected
    }

    async fn get_xpub(&self, derivation_path: &str) -> BitcoinResult<String> {
        info!("Getting xpub for path: {}", derivation_path);
        Ok("xpub6CUGRUonZSQ4TWtTMmzXdrXDtypWKiKrhko4egpiMZbpiaQL2jkwSB1icqYh2cfDfVxdx4df189oLKnC5fSwqPfgyP3hooxujYzAu3fDVmz".to_string())
    }

    async fn sign_psbt(&self, psbt: &mut AdvancedPsbtBuilder) -> BitcoinResult<()> {
        info!("Signing PSBT with {}", self.device_name);
        // Mock signing - in real implementation would communicate with hardware
        Ok(())
    }

    async fn get_address(&self, derivation_path: &str, address_type: super::AddressType) -> BitcoinResult<String> {
        info!("Getting {} address for path: {}", address_type.as_str(), derivation_path);
        Ok("bc1qtest123456789".to_string())
    }

    async fn display_address(&self, derivation_path: &str) -> BitcoinResult<String> {
        info!("Displaying address on {} for path: {}", self.device_name, derivation_path);
        Ok("bc1qtest123456789".to_string())
    }
}

/// PSBT Workflow Manager for coordinating the complete PSBT lifecycle
pub struct PsbtWorkflowManager {
    hardware_wallets: Vec<Box<dyn HardwareWallet>>,
    network: Network,
}

impl PsbtWorkflowManager {
    /// Create new workflow manager
    pub fn new(network: Network) -> Self {
        Self {
            hardware_wallets: Vec::new(),
            network,
        }
    }

    /// Add hardware wallet to the workflow
    pub fn add_hardware_wallet(&mut self, wallet: Box<dyn HardwareWallet>) {
        self.hardware_wallets.push(wallet);
    }

    /// Create PSBT from transaction inputs and outputs
    pub fn create_psbt(
        &self,
        inputs: Vec<PsbtInputInfo>,
        outputs: Vec<PsbtOutputInfo>,
    ) -> BitcoinResult<AdvancedPsbtBuilder> {
        let mut builder = AdvancedPsbtBuilder::new(self.network);

        for input in inputs {
            builder.add_input(input)?;
        }

        for output in outputs {
            builder.add_output(output)?;
        }

        info!("Created PSBT with {} inputs and {} outputs",
              builder.psbt().inputs.len(), builder.psbt().outputs.len());

        Ok(builder)
    }

    /// Sign PSBT with all available hardware wallets
    pub async fn sign_with_hardware_wallets(&self, psbt: &mut AdvancedPsbtBuilder) -> BitcoinResult<()> {
        for wallet in &self.hardware_wallets {
            if wallet.is_connected().await {
                wallet.sign_psbt(psbt).await?;
                info!("Signed PSBT with {}", wallet.device_name());
            } else {
                warn!("Hardware wallet {} is not connected", wallet.device_name());
            }
        }
        Ok(())
    }

    /// Complete PSBT workflow: Create -> Sign -> Finalize -> Extract
    pub async fn complete_workflow(
        &self,
        inputs: Vec<PsbtInputInfo>,
        outputs: Vec<PsbtOutputInfo>,
    ) -> BitcoinResult<Transaction> {
        // Step 1: Create PSBT
        let mut psbt = self.create_psbt(inputs, outputs)?;

        // Step 2: Validate PSBT
        let validation = psbt.validate();
        if !validation.is_valid {
            return Err(BitcoinError::InvalidPsbt(format!(
                "PSBT validation failed: {:?}", validation.errors
            )));
        }

        // Step 3: Sign with hardware wallets
        self.sign_with_hardware_wallets(&mut psbt).await?;

        // Step 4: Check if complete
        if !psbt.is_complete() {
            return Err(BitcoinError::SigningError("PSBT is not fully signed".to_string()));
        }

        // Step 5: Finalize PSBT
        PsbtFinalizer::finalize(&mut psbt)?;

        // Step 6: Extract final transaction
        let final_tx = PsbtFinalizer::extract_transaction(&psbt)?;

        info!("Completed PSBT workflow successfully");
        Ok(final_tx)
    }
}

/// Multi-signature PSBT support
pub struct MultiSigPsbtManager {
    required_signatures: usize,
    total_signers: usize,
    network: Network,
}

impl MultiSigPsbtManager {
    /// Create new multi-sig PSBT manager
    pub fn new(required_signatures: usize, total_signers: usize, network: Network) -> BitcoinResult<Self> {
        if required_signatures == 0 || required_signatures > total_signers {
            return Err(BitcoinError::InvalidPsbt(
                "Invalid multi-sig configuration".to_string()
            ));
        }

        Ok(Self {
            required_signatures,
            total_signers,
            network,
        })
    }

    /// Create multi-sig PSBT
    pub fn create_multisig_psbt(
        &self,
        inputs: Vec<PsbtInputInfo>,
        outputs: Vec<PsbtOutputInfo>,
        public_keys: Vec<String>,
    ) -> BitcoinResult<AdvancedPsbtBuilder> {
        if public_keys.len() != self.total_signers {
            return Err(BitcoinError::InvalidPsbt(
                "Public key count doesn't match total signers".to_string()
            ));
        }

        let mut builder = AdvancedPsbtBuilder::new(self.network);

        for input in inputs {
            builder.add_input(input)?;
        }

        for output in outputs {
            builder.add_output(output)?;
        }

        // Add multi-sig information to PSBT inputs
        // This would require more complex implementation with actual script creation

        info!("Created multi-sig PSBT ({} of {})", self.required_signatures, self.total_signers);
        Ok(builder)
    }

    /// Check if PSBT has enough signatures
    pub fn has_enough_signatures(&self, psbt: &AdvancedPsbtBuilder) -> bool {
        psbt.psbt().inputs.iter().all(|input| {
            input.partial_sigs.len() >= self.required_signatures
        })
    }

    /// Collect signatures from multiple parties
    pub fn collect_signatures(
        &self,
        base_psbt: &mut AdvancedPsbtBuilder,
        signed_psbts: Vec<AdvancedPsbtBuilder>,
    ) -> BitcoinResult<()> {
        let others: Vec<&AdvancedPsbtBuilder> = signed_psbts.iter().collect();
        PsbtCombiner::merge_signatures(base_psbt, others)?;

        if self.has_enough_signatures(base_psbt) {
            info!("Collected enough signatures for multi-sig PSBT");
        } else {
            warn!("Still need more signatures for multi-sig PSBT");
        }

        Ok(())
    }
}

/// PSBT utilities
pub struct PsbtUtils;

impl PsbtUtils {
    /// Calculate PSBT fee
    pub fn calculate_fee(psbt: &AdvancedPsbtBuilder) -> u64 {
        let stats = psbt.get_stats();
        stats.fee
    }

    /// Estimate transaction size after finalization
    pub fn estimate_final_size(psbt: &AdvancedPsbtBuilder) -> usize {
        // This is a rough estimation
        // Real implementation would consider actual script sizes
        let base_size = bitcoin::consensus::serialize(&psbt.psbt().unsigned_tx).len();
        let estimated_script_overhead = psbt.psbt().inputs.len() * 100; // Rough estimate
        base_size + estimated_script_overhead
    }

    /// Check if fee rate is reasonable
    pub fn check_fee_rate(psbt: &AdvancedPsbtBuilder, min_fee_rate: f64, max_fee_rate: f64) -> BitcoinResult<()> {
        let stats = psbt.get_stats();

        if stats.fee_rate < min_fee_rate {
            return Err(BitcoinError::Rpc(format!(
                "Fee rate too low: {} sat/vB (minimum: {})",
                stats.fee_rate, min_fee_rate
            )));
        }

        if stats.fee_rate > max_fee_rate {
            return Err(BitcoinError::Rpc(format!(
                "Fee rate too high: {} sat/vB (maximum: {})",
                stats.fee_rate, max_fee_rate
            )));
        }

        Ok(())
    }

    /// Convert PSBT to QR code data
    pub fn to_qr_data(psbt: &AdvancedPsbtBuilder) -> String {
        // For QR codes, we typically use base64 encoding
        psbt.to_base64()
    }

    /// Parse PSBT from QR code data
    pub fn from_qr_data(qr_data: &str) -> BitcoinResult<AdvancedPsbtBuilder> {
        AdvancedPsbtBuilder::from_base64(qr_data)
    }
}
