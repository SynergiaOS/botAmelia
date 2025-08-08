//! Bitcoin Security Validator Module
//! 
//! This module provides security validation for Bitcoin transactions
//! before signing to prevent common attacks and mistakes.

use super::{BitcoinError, BitcoinResult, Network};
use bitcoin::{Address, Amount, Transaction, TxOut};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::str::FromStr;
use tracing::{debug, error, info, warn};

/// Security validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Maximum fee rate in sat/vB
    pub max_fee_rate: f64,
    /// Maximum absolute fee in satoshis
    pub max_absolute_fee: u64,
    /// Minimum output amount (dust threshold)
    pub min_output_amount: u64,
    /// Maximum single output amount
    pub max_output_amount: u64,
    /// Maximum total transaction amount
    pub max_total_amount: u64,
    /// Blocked addresses (known malicious addresses)
    pub blocked_addresses: HashSet<String>,
    /// Require confirmation for large amounts
    pub large_amount_threshold: u64,
    /// Enable address validation
    pub validate_addresses: bool,
    /// Enable fee validation
    pub validate_fees: bool,
    /// Enable amount validation
    pub validate_amounts: bool,
}

/// Security validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether transaction is valid
    pub is_valid: bool,
    /// Security level (Low, Medium, High, Critical)
    pub security_level: SecurityLevel,
    /// Validation errors
    pub errors: Vec<String>,
    /// Validation warnings
    pub warnings: Vec<String>,
    /// Recommendations
    pub recommendations: Vec<String>,
    /// Requires manual confirmation
    pub requires_confirmation: bool,
}

/// Security level assessment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecurityLevel {
    /// Low risk transaction
    Low,
    /// Medium risk transaction
    Medium,
    /// High risk transaction
    High,
    /// Critical risk transaction (should be blocked)
    Critical,
}

/// Transaction risk factors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactors {
    /// High fee rate
    pub high_fee_rate: bool,
    /// Large amount
    pub large_amount: bool,
    /// Suspicious address
    pub suspicious_address: bool,
    /// Unusual output pattern
    pub unusual_pattern: bool,
    /// Dust outputs
    pub dust_outputs: bool,
    /// Round number amounts (potential indicator of manual entry)
    pub round_amounts: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            max_fee_rate: 1000.0, // 1000 sat/vB
            max_absolute_fee: 1_000_000, // 0.01 BTC
            min_output_amount: 546, // Standard dust threshold
            max_output_amount: 100_000_000, // 1 BTC
            max_total_amount: 1_000_000_000, // 10 BTC
            blocked_addresses: HashSet::new(),
            large_amount_threshold: 10_000_000, // 0.1 BTC
            validate_addresses: true,
            validate_fees: true,
            validate_amounts: true,
        }
    }
}

impl SecurityLevel {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            SecurityLevel::Low => "Low",
            SecurityLevel::Medium => "Medium",
            SecurityLevel::High => "High",
            SecurityLevel::Critical => "Critical",
        }
    }

    /// Check if level requires confirmation
    pub fn requires_confirmation(&self) -> bool {
        matches!(self, SecurityLevel::High | SecurityLevel::Critical)
    }
}

/// Bitcoin Security Validator
pub struct SecurityValidator {
    config: SecurityConfig,
    network: Network,
}

impl SecurityValidator {
    /// Create new security validator
    pub fn new(network: Network) -> Self {
        Self {
            config: SecurityConfig::default(),
            network,
        }
    }

    /// Create security validator with custom config
    pub fn with_config(network: Network, config: SecurityConfig) -> Self {
        Self { config, network }
    }

    /// Validate transaction before signing
    pub fn validate_transaction(
        &self,
        transaction: &Transaction,
        input_amounts: &[u64],
    ) -> BitcoinResult<ValidationResult> {
        let mut result = ValidationResult {
            is_valid: true,
            security_level: SecurityLevel::Low,
            errors: Vec::new(),
            warnings: Vec::new(),
            recommendations: Vec::new(),
            requires_confirmation: false,
        };

        let mut risk_factors = RiskFactors {
            high_fee_rate: false,
            large_amount: false,
            suspicious_address: false,
            unusual_pattern: false,
            dust_outputs: false,
            round_amounts: false,
        };

        // Calculate transaction metrics
        let total_input = input_amounts.iter().sum::<u64>();
        let total_output: u64 = transaction.output.iter().map(|out| out.value.to_sat()).sum();
        let fee = total_input.saturating_sub(total_output);
        let tx_size = bitcoin::consensus::serialize(transaction).len();
        let fee_rate = if tx_size > 0 { fee as f64 / tx_size as f64 } else { 0.0 };

        // Validate fees
        if self.config.validate_fees {
            self.validate_fees(&mut result, &mut risk_factors, fee, fee_rate)?;
        }

        // Validate amounts
        if self.config.validate_amounts {
            self.validate_amounts(&mut result, &mut risk_factors, &transaction.output, total_output)?;
        }

        // Validate addresses
        if self.config.validate_addresses {
            self.validate_addresses(&mut result, &mut risk_factors, &transaction.output)?;
        }

        // Validate transaction patterns
        self.validate_patterns(&mut result, &mut risk_factors, transaction)?;

        // Assess overall security level
        result.security_level = self.assess_security_level(&risk_factors);
        result.requires_confirmation = result.security_level.requires_confirmation();

        // Add recommendations based on risk factors
        self.add_recommendations(&mut result, &risk_factors);

        info!(
            "Transaction validation completed: {} ({})",
            result.security_level.as_str(),
            if result.is_valid { "VALID" } else { "INVALID" }
        );

        Ok(result)
    }

    /// Validate transaction fees
    fn validate_fees(
        &self,
        result: &mut ValidationResult,
        risk_factors: &mut RiskFactors,
        fee: u64,
        fee_rate: f64,
    ) -> BitcoinResult<()> {
        // Check absolute fee limit
        if fee > self.config.max_absolute_fee {
            result.errors.push(format!(
                "Fee too high: {} sat (max: {} sat)",
                fee, self.config.max_absolute_fee
            ));
            result.is_valid = false;
        }

        // Check fee rate limit
        if fee_rate > self.config.max_fee_rate {
            result.errors.push(format!(
                "Fee rate too high: {:.2} sat/vB (max: {:.2} sat/vB)",
                fee_rate, self.config.max_fee_rate
            ));
            result.is_valid = false;
            risk_factors.high_fee_rate = true;
        }

        // Warn about high fees
        if fee_rate > self.config.max_fee_rate * 0.5 {
            result.warnings.push(format!(
                "High fee rate: {:.2} sat/vB",
                fee_rate
            ));
            risk_factors.high_fee_rate = true;
        }

        // Warn about very low fees
        if fee_rate < 1.0 {
            result.warnings.push(format!(
                "Very low fee rate: {:.2} sat/vB (may not confirm quickly)",
                fee_rate
            ));
        }

        Ok(())
    }

    /// Validate transaction amounts
    fn validate_amounts(
        &self,
        result: &mut ValidationResult,
        risk_factors: &mut RiskFactors,
        outputs: &[TxOut],
        total_output: u64,
    ) -> BitcoinResult<()> {
        // Check total amount limit
        if total_output > self.config.max_total_amount {
            result.errors.push(format!(
                "Total amount too high: {} sat (max: {} sat)",
                total_output, self.config.max_total_amount
            ));
            result.is_valid = false;
        }

        // Check large amount threshold
        if total_output > self.config.large_amount_threshold {
            result.warnings.push(format!(
                "Large transaction amount: {} sat",
                total_output
            ));
            risk_factors.large_amount = true;
        }

        // Check individual outputs
        for (i, output) in outputs.iter().enumerate() {
            let amount = output.value.to_sat();

            // Check dust threshold
            if amount < self.config.min_output_amount {
                result.warnings.push(format!(
                    "Output {} below dust threshold: {} sat",
                    i, amount
                ));
                risk_factors.dust_outputs = true;
            }

            // Check maximum output amount
            if amount > self.config.max_output_amount {
                result.errors.push(format!(
                    "Output {} amount too high: {} sat (max: {} sat)",
                    i, amount, self.config.max_output_amount
                ));
                result.is_valid = false;
            }

            // Check for round numbers (potential manual entry)
            if amount % 100_000_000 == 0 || amount % 10_000_000 == 0 {
                risk_factors.round_amounts = true;
            }
        }

        Ok(())
    }

    /// Validate output addresses
    fn validate_addresses(
        &self,
        result: &mut ValidationResult,
        risk_factors: &mut RiskFactors,
        outputs: &[TxOut],
    ) -> BitcoinResult<()> {
        for (i, output) in outputs.iter().enumerate() {
            // Try to parse address from script
            if let Ok(address) = Address::from_script(&output.script_pubkey, self.network.into()) {
                let address_str = address.to_string();

                // Check blocked addresses
                if self.config.blocked_addresses.contains(&address_str) {
                    result.errors.push(format!(
                        "Output {} sends to blocked address: {}",
                        i, address_str
                    ));
                    result.is_valid = false;
                    risk_factors.suspicious_address = true;
                }

                // Additional address validation could be added here
                // (e.g., checking against known exchange addresses, etc.)
            }
        }

        Ok(())
    }

    /// Validate transaction patterns
    fn validate_patterns(
        &self,
        result: &mut ValidationResult,
        risk_factors: &mut RiskFactors,
        transaction: &Transaction,
    ) -> BitcoinResult<()> {
        // Check for unusual patterns
        let output_count = transaction.output.len();
        let input_count = transaction.input.len();

        // Many outputs might indicate mixing or distribution
        if output_count > 20 {
            result.warnings.push(format!(
                "Transaction has many outputs: {}",
                output_count
            ));
            risk_factors.unusual_pattern = true;
        }

        // Many inputs might indicate consolidation
        if input_count > 50 {
            result.warnings.push(format!(
                "Transaction has many inputs: {}",
                input_count
            ));
            risk_factors.unusual_pattern = true;
        }

        // Check for exact amount matches (potential indicator of specific attack patterns)
        let amounts: Vec<u64> = transaction.output.iter().map(|out| out.value.to_sat()).collect();
        for (i, &amount1) in amounts.iter().enumerate() {
            for (j, &amount2) in amounts.iter().enumerate() {
                if i != j && amount1 == amount2 && amount1 > 1_000_000 {
                    result.warnings.push(format!(
                        "Outputs {} and {} have identical amounts: {} sat",
                        i, j, amount1
                    ));
                    risk_factors.unusual_pattern = true;
                    break;
                }
            }
        }

        Ok(())
    }

    /// Assess overall security level based on risk factors
    fn assess_security_level(&self, risk_factors: &RiskFactors) -> SecurityLevel {
        let mut score = 0;

        if risk_factors.high_fee_rate { score += 2; }
        if risk_factors.large_amount { score += 1; }
        if risk_factors.suspicious_address { score += 3; }
        if risk_factors.unusual_pattern { score += 1; }
        if risk_factors.dust_outputs { score += 1; }
        if risk_factors.round_amounts { score += 1; }

        match score {
            0..=1 => SecurityLevel::Low,
            2..=3 => SecurityLevel::Medium,
            4..=5 => SecurityLevel::High,
            _ => SecurityLevel::Critical,
        }
    }

    /// Add recommendations based on risk factors
    fn add_recommendations(&self, result: &mut ValidationResult, risk_factors: &RiskFactors) {
        if risk_factors.high_fee_rate {
            result.recommendations.push("Consider reducing the fee rate to save on transaction costs".to_string());
        }

        if risk_factors.large_amount {
            result.recommendations.push("Consider splitting large transactions into smaller amounts".to_string());
        }

        if risk_factors.dust_outputs {
            result.recommendations.push("Consider consolidating dust outputs to reduce transaction size".to_string());
        }

        if risk_factors.unusual_pattern {
            result.recommendations.push("Review transaction pattern for potential optimization".to_string());
        }

        if result.security_level == SecurityLevel::High || result.security_level == SecurityLevel::Critical {
            result.recommendations.push("Manual review recommended before signing".to_string());
        }
    }

    /// Update security configuration
    pub fn update_config(&mut self, config: SecurityConfig) {
        self.config = config;
        info!("Security configuration updated");
    }

    /// Add blocked address
    pub fn add_blocked_address(&mut self, address: String) {
        self.config.blocked_addresses.insert(address.clone());
        warn!("Added blocked address: {}", address);
    }

    /// Remove blocked address
    pub fn remove_blocked_address(&mut self, address: &str) -> bool {
        let removed = self.config.blocked_addresses.remove(address);
        if removed {
            info!("Removed blocked address: {}", address);
        }
        removed
    }

    /// Get current configuration
    pub fn config(&self) -> &SecurityConfig {
        &self.config
    }
}
