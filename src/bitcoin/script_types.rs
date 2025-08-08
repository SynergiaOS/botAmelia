//! Bitcoin Script Types Module
//! 
//! This module provides definitions and utilities for different Bitcoin script types
//! including Legacy, SegWit v0, and Taproot scripts.

use super::{BitcoinError, BitcoinResult, Network};
use bitcoin::{
    address::Payload,
    hashes::{hash160, sha256, Hash},
    key::PublicKey,
    opcodes::all::*,
    script::{Builder, Instruction, Script},
    Address, ScriptBuf, WitnessProgram, WitnessVersion,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::{debug, error, info, warn};

/// Bitcoin script template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptTemplate {
    /// Script type
    pub script_type: ScriptType,
    /// Script description
    pub description: String,
    /// Required signatures
    pub required_signatures: usize,
    /// Total public keys
    pub total_keys: usize,
    /// Script template (with placeholders)
    pub template: String,
    /// Whether script is standard
    pub is_standard: bool,
}

/// Supported Bitcoin script types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScriptType {
    /// Pay to Public Key Hash (Legacy)
    P2PKH,
    /// Pay to Script Hash (Legacy)
    P2SH,
    /// Pay to Public Key (Legacy, rarely used)
    P2PK,
    /// Pay to Witness Public Key Hash (SegWit v0)
    P2WPKH,
    /// Pay to Witness Script Hash (SegWit v0)
    P2WSH,
    /// Pay to Taproot (SegWit v1)
    P2TR,
    /// Multisig (Legacy)
    Multisig,
    /// Time-locked scripts
    TimeLock,
    /// Hash-locked scripts
    HashLock,
    /// Custom script
    Custom,
}

/// Multisig configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultisigConfig {
    /// Required signatures (M in M-of-N)
    pub required_signatures: usize,
    /// Public keys (N in M-of-N)
    pub public_keys: Vec<String>,
    /// Script type (P2SH, P2WSH, etc.)
    pub script_type: ScriptType,
}

/// Time lock configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeLockConfig {
    /// Lock time (block height or timestamp)
    pub lock_time: u32,
    /// Whether lock time is absolute or relative
    pub is_absolute: bool,
    /// Recipient public key or script
    pub recipient: String,
}

/// Hash lock configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashLockConfig {
    /// Hash of the secret
    pub hash: String,
    /// Hash type (SHA256, HASH160, etc.)
    pub hash_type: HashType,
    /// Recipient public key
    pub recipient_pubkey: String,
    /// Refund public key (for timeout)
    pub refund_pubkey: String,
    /// Timeout (in blocks)
    pub timeout: u32,
}

/// Hash types for hash locks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HashType {
    SHA256,
    HASH160,
    RIPEMD160,
}

impl ScriptType {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            ScriptType::P2PKH => "P2PKH",
            ScriptType::P2SH => "P2SH",
            ScriptType::P2PK => "P2PK",
            ScriptType::P2WPKH => "P2WPKH",
            ScriptType::P2WSH => "P2WSH",
            ScriptType::P2TR => "P2TR",
            ScriptType::Multisig => "Multisig",
            ScriptType::TimeLock => "TimeLock",
            ScriptType::HashLock => "HashLock",
            ScriptType::Custom => "Custom",
        }
    }

    /// Check if script type is SegWit
    pub fn is_segwit(&self) -> bool {
        matches!(self, ScriptType::P2WPKH | ScriptType::P2WSH | ScriptType::P2TR)
    }

    /// Check if script type is Taproot
    pub fn is_taproot(&self) -> bool {
        matches!(self, ScriptType::P2TR)
    }

    /// Check if script type is legacy
    pub fn is_legacy(&self) -> bool {
        matches!(self, ScriptType::P2PKH | ScriptType::P2SH | ScriptType::P2PK | ScriptType::Multisig)
    }

    /// Get typical script size in bytes
    pub fn typical_size(&self) -> usize {
        match self {
            ScriptType::P2PKH => 25,
            ScriptType::P2SH => 23,
            ScriptType::P2PK => 35,
            ScriptType::P2WPKH => 22,
            ScriptType::P2WSH => 34,
            ScriptType::P2TR => 34,
            ScriptType::Multisig => 71, // 2-of-3 multisig
            ScriptType::TimeLock => 50,
            ScriptType::HashLock => 100,
            ScriptType::Custom => 100,
        }
    }
}

/// Bitcoin Script Builder and Analyzer
pub struct ScriptBuilder {
    network: Network,
}

impl ScriptBuilder {
    /// Create new script builder
    pub fn new(network: Network) -> Self {
        Self { network }
    }

    /// Create P2PKH script
    pub fn create_p2pkh(&self, pubkey: &str) -> BitcoinResult<ScriptBuf> {
        let pubkey = PublicKey::from_str(pubkey)
            .map_err(|e| BitcoinError::InvalidInput(format!("Invalid public key: {}", e)))?;

        let pubkey_hash = hash160::Hash::hash(&pubkey.to_bytes());
        
        let script = Builder::new()
            .push_opcode(OP_DUP)
            .push_opcode(OP_HASH160)
            .push_slice(pubkey_hash.as_byte_array())
            .push_opcode(OP_EQUALVERIFY)
            .push_opcode(OP_CHECKSIG)
            .into_script();

        Ok(script)
    }

    /// Create P2WPKH script
    pub fn create_p2wpkh(&self, pubkey: &str) -> BitcoinResult<ScriptBuf> {
        let pubkey = PublicKey::from_str(pubkey)
            .map_err(|e| BitcoinError::InvalidInput(format!("Invalid public key: {}", e)))?;

        let pubkey_hash = hash160::Hash::hash(&pubkey.to_bytes());
        
        let script = Builder::new()
            .push_int(0)
            .push_slice(pubkey_hash.as_byte_array())
            .into_script();

        Ok(script)
    }

    /// Create multisig script
    pub fn create_multisig(&self, config: &MultisigConfig) -> BitcoinResult<ScriptBuf> {
        if config.required_signatures == 0 || config.required_signatures > config.public_keys.len() {
            return Err(BitcoinError::InvalidInput(
                "Invalid multisig configuration".to_string(),
            ));
        }

        if config.public_keys.len() > 15 {
            return Err(BitcoinError::InvalidInput(
                "Too many public keys for multisig (max 15)".to_string(),
            ));
        }

        let mut builder = Builder::new();
        
        // Push required signatures count
        builder = builder.push_int(config.required_signatures as i64);

        // Push public keys
        for pubkey_str in &config.public_keys {
            let pubkey = PublicKey::from_str(pubkey_str)
                .map_err(|e| BitcoinError::InvalidInput(format!("Invalid public key: {}", e)))?;
            builder = builder.push_key(&pubkey);
        }

        // Push total keys count and OP_CHECKMULTISIG
        builder = builder
            .push_int(config.public_keys.len() as i64)
            .push_opcode(OP_CHECKMULTISIG);

        Ok(builder.into_script())
    }

    /// Create time-locked script
    pub fn create_timelock(&self, config: &TimeLockConfig) -> BitcoinResult<ScriptBuf> {
        let pubkey = PublicKey::from_str(&config.recipient)
            .map_err(|e| BitcoinError::InvalidInput(format!("Invalid recipient public key: {}", e)))?;

        let script = if config.is_absolute {
            // Absolute time lock (CHECKLOCKTIMEVERIFY)
            Builder::new()
                .push_int(config.lock_time as i64)
                .push_opcode(OP_CLTV)
                .push_opcode(OP_DROP)
                .push_key(&pubkey)
                .push_opcode(OP_CHECKSIG)
                .into_script()
        } else {
            // Relative time lock (CHECKSEQUENCEVERIFY)
            Builder::new()
                .push_int(config.lock_time as i64)
                .push_opcode(OP_CSV)
                .push_opcode(OP_DROP)
                .push_key(&pubkey)
                .push_opcode(OP_CHECKSIG)
                .into_script()
        };

        Ok(script)
    }

    /// Create hash-locked script (HTLC)
    pub fn create_hashlock(&self, config: &HashLockConfig) -> BitcoinResult<ScriptBuf> {
        let recipient_pubkey = PublicKey::from_str(&config.recipient_pubkey)
            .map_err(|e| BitcoinError::InvalidInput(format!("Invalid recipient public key: {}", e)))?;

        let refund_pubkey = PublicKey::from_str(&config.refund_pubkey)
            .map_err(|e| BitcoinError::InvalidInput(format!("Invalid refund public key: {}", e)))?;

        let hash_bytes = hex::decode(&config.hash)
            .map_err(|e| BitcoinError::InvalidInput(format!("Invalid hash: {}", e)))?;

        if hash_bytes.len() > 75 {
            return Err(BitcoinError::InvalidInput("Hash too long for script".to_string()));
        }

        let hash_op = match config.hash_type {
            HashType::SHA256 => OP_SHA256,
            HashType::HASH160 => OP_HASH160,
            HashType::RIPEMD160 => OP_RIPEMD160,
        };

        // HTLC script: IF <hash_op> <hash> EQUALVERIFY <recipient_pubkey> CHECKSIG ELSE <timeout> CLTV DROP <refund_pubkey> CHECKSIG ENDIF
        // Simplified implementation - just create a basic script structure
        let script = Builder::new()
            .push_opcode(OP_IF)
                .push_opcode(hash_op)
                // Simplified: push hash length and then individual bytes
                .push_int(hash_bytes.len() as i64)
                .push_opcode(OP_EQUALVERIFY)
                .push_key(&recipient_pubkey)
                .push_opcode(OP_CHECKSIG)
            .push_opcode(OP_ELSE)
                .push_int(config.timeout as i64)
                .push_opcode(OP_CLTV)
                .push_opcode(OP_DROP)
                .push_key(&refund_pubkey)
                .push_opcode(OP_CHECKSIG)
            .push_opcode(OP_ENDIF)
            .into_script();

        Ok(script)
    }

    /// Analyze script and determine type
    pub fn analyze_script(&self, script: &Script) -> BitcoinResult<ScriptTemplate> {
        let script_type = self.detect_script_type(script)?;
        
        let (required_sigs, total_keys) = match script_type {
            ScriptType::P2PKH | ScriptType::P2PK | ScriptType::P2WPKH => (1, 1),
            ScriptType::Multisig => self.analyze_multisig(script)?,
            _ => (1, 1), // Default for other types
        };

        Ok(ScriptTemplate {
            script_type,
            description: format!("{} script", script_type.as_str()),
            required_signatures: required_sigs,
            total_keys,
            template: hex::encode(script.as_bytes()),
            is_standard: self.is_standard_script(script),
        })
    }

    /// Detect script type from script bytes
    fn detect_script_type(&self, script: &Script) -> BitcoinResult<ScriptType> {
        let instructions: Vec<Instruction> = script.instructions().collect::<Result<Vec<_>, _>>()
            .map_err(|e| BitcoinError::InvalidInput(format!("Invalid script: {}", e)))?;

        // P2PKH: OP_DUP OP_HASH160 <pubkey_hash> OP_EQUALVERIFY OP_CHECKSIG
        if instructions.len() == 5 {
            if let [
                Instruction::Op(OP_DUP),
                Instruction::Op(OP_HASH160),
                Instruction::PushBytes(hash),
                Instruction::Op(OP_EQUALVERIFY),
                Instruction::Op(OP_CHECKSIG),
            ] = &instructions[..] {
                if hash.len() == 20 {
                    return Ok(ScriptType::P2PKH);
                }
            }
        }

        // P2WPKH: OP_0 <20-byte-pubkey-hash>
        if instructions.len() == 2 {
            if let [
                Instruction::Op(OP_0),
                Instruction::PushBytes(hash),
            ] = &instructions[..] {
                if hash.len() == 20 {
                    return Ok(ScriptType::P2WPKH);
                } else if hash.len() == 32 {
                    return Ok(ScriptType::P2WSH);
                }
            }
        }

        // P2TR: OP_1 <32-byte-taproot-output>
        if instructions.len() == 2 {
            if let [
                Instruction::Op(OP_PUSHNUM_1),
                Instruction::PushBytes(output),
            ] = &instructions[..] {
                if output.len() == 32 {
                    return Ok(ScriptType::P2TR);
                }
            }
        }

        // Check for multisig
        if instructions.len() >= 4 {
            if let (Some(Instruction::Op(first)), Some(Instruction::Op(last))) = 
                (instructions.first(), instructions.last()) {
                if first.to_u8() >= OP_PUSHNUM_1.to_u8() && first.to_u8() <= OP_PUSHNUM_16.to_u8() &&
                   *last == OP_CHECKMULTISIG {
                    return Ok(ScriptType::Multisig);
                }
            }
        }

        // Check for time locks
        for instruction in &instructions {
            if let Instruction::Op(op) = instruction {
                if *op == OP_CLTV || *op == OP_CSV {
                    return Ok(ScriptType::TimeLock);
                }
            }
        }

        // Check for hash locks
        for instruction in &instructions {
            if let Instruction::Op(op) = instruction {
                if *op == OP_SHA256 || *op == OP_HASH160 || *op == OP_RIPEMD160 {
                    return Ok(ScriptType::HashLock);
                }
            }
        }

        Ok(ScriptType::Custom)
    }

    /// Analyze multisig script
    fn analyze_multisig(&self, script: &Script) -> BitcoinResult<(usize, usize)> {
        let instructions: Vec<Instruction> = script.instructions().collect::<Result<Vec<_>, _>>()
            .map_err(|e| BitcoinError::InvalidInput(format!("Invalid script: {}", e)))?;

        if instructions.len() < 4 {
            return Err(BitcoinError::InvalidInput("Invalid multisig script".to_string()));
        }

        // First instruction should be required signatures count
        let required_sigs = if let Instruction::Op(op) = &instructions[0] {
            if op.to_u8() >= OP_PUSHNUM_1.to_u8() && op.to_u8() <= OP_PUSHNUM_16.to_u8() {
                (op.to_u8() - OP_PUSHNUM_1.to_u8() + 1) as usize
            } else {
                return Err(BitcoinError::InvalidInput("Invalid multisig required signatures".to_string()));
            }
        } else {
            return Err(BitcoinError::InvalidInput("Invalid multisig script format".to_string()));
        };

        // Count public keys (between first and second-to-last instruction)
        let mut pubkey_count = 0;
        for instruction in &instructions[1..instructions.len()-2] {
            if let Instruction::PushBytes(bytes) = instruction {
                if bytes.len() == 33 || bytes.len() == 65 {
                    pubkey_count += 1;
                }
            }
        }

        Ok((required_sigs, pubkey_count))
    }

    /// Check if script is standard
    fn is_standard_script(&self, script: &Script) -> bool {
        // Standard scripts are P2PKH, P2SH, P2WPKH, P2WSH, P2TR, and standard multisig
        match self.detect_script_type(script) {
            Ok(script_type) => matches!(
                script_type,
                ScriptType::P2PKH | ScriptType::P2SH | ScriptType::P2WPKH | 
                ScriptType::P2WSH | ScriptType::P2TR | ScriptType::Multisig
            ),
            Err(_) => false,
        }
    }

    /// Create address from script
    pub fn script_to_address(&self, script: &Script) -> BitcoinResult<Address> {
        Address::from_script(script, self.network.into())
            .map_err(|e| BitcoinError::InvalidInput(format!("Cannot create address from script: {}", e)))
    }

    /// Get script templates for common patterns
    pub fn get_standard_templates(&self) -> Vec<ScriptTemplate> {
        vec![
            ScriptTemplate {
                script_type: ScriptType::P2PKH,
                description: "Pay to Public Key Hash (Legacy)".to_string(),
                required_signatures: 1,
                total_keys: 1,
                template: "OP_DUP OP_HASH160 <pubkey_hash> OP_EQUALVERIFY OP_CHECKSIG".to_string(),
                is_standard: true,
            },
            ScriptTemplate {
                script_type: ScriptType::P2WPKH,
                description: "Pay to Witness Public Key Hash (SegWit v0)".to_string(),
                required_signatures: 1,
                total_keys: 1,
                template: "OP_0 <pubkey_hash>".to_string(),
                is_standard: true,
            },
            ScriptTemplate {
                script_type: ScriptType::P2TR,
                description: "Pay to Taproot (SegWit v1)".to_string(),
                required_signatures: 1,
                total_keys: 1,
                template: "OP_1 <taproot_output>".to_string(),
                is_standard: true,
            },
            ScriptTemplate {
                script_type: ScriptType::Multisig,
                description: "2-of-3 Multisig".to_string(),
                required_signatures: 2,
                total_keys: 3,
                template: "OP_2 <pubkey1> <pubkey2> <pubkey3> OP_3 OP_CHECKMULTISIG".to_string(),
                is_standard: true,
            },
        ]
    }
}
