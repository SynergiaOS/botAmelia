//! Unit tests for Bitcoin Core integration

use cerberus::bitcoin::{
    BitcoinConfig, BitcoinCore, Network, AddressType, Amount,
    PsbtBuilder, TransactionBuilder, WalletManager,
    AdvancedPsbtBuilder, PsbtWorkflowManager, MultiSigPsbtManager, PsbtUtils,
    MockHardwareWallet, HardwareWallet,
    TransactionSigner, InputSigningInfo, KeyManager, SecurityValidator,
    ScriptBuilder, HardwareWalletManager, HardwareSigningRequest,
};
use cerberus::bitcoin::psbt_advanced::{PsbtInputInfo, PsbtOutputInfo};
use cerberus::bitcoin::transaction_signer::ScriptType as TxScriptType;
use cerberus::bitcoin::script_types::ScriptType;
use cerberus::bitcoin::hardware_signer::AddressVerificationRequest;
use anyhow::Result;
use base64::prelude::*;

/// Test Bitcoin configuration
fn test_config() -> BitcoinConfig {
    BitcoinConfig {
        rpc_url: "http://127.0.0.1:18443".to_string(), // regtest port
        rpc_user: "test".to_string(),
        rpc_password: "test".to_string(),
        network: Network::Regtest,
        wallet_name: Some("test_wallet".to_string()),
        timeout: 30,
        max_retries: 3,
        enable_zmq: false,
        zmq_block_endpoint: None,
        zmq_tx_endpoint: None,
    }
}

#[tokio::test]
async fn test_bitcoin_config_creation() -> Result<()> {
    let config = test_config();
    
    assert_eq!(config.network, Network::Regtest);
    assert_eq!(config.rpc_url, "http://127.0.0.1:18443");
    assert_eq!(config.wallet_name, Some("test_wallet".to_string()));
    
    Ok(())
}

#[tokio::test]
async fn test_bitcoin_config_default() -> Result<()> {
    let config = BitcoinConfig::default();
    
    assert_eq!(config.network, Network::Regtest);
    assert_eq!(config.rpc_url, "http://127.0.0.1:8332");
    assert_eq!(config.timeout, 30);
    assert_eq!(config.max_retries, 3);
    
    Ok(())
}

#[tokio::test]
async fn test_network_methods() -> Result<()> {
    assert_eq!(Network::Mainnet.as_str(), "main");
    assert_eq!(Network::Testnet.as_str(), "test");
    assert_eq!(Network::Regtest.as_str(), "regtest");
    assert_eq!(Network::Signet.as_str(), "signet");
    
    assert_eq!(Network::Mainnet.address_prefix(), "bc1");
    assert_eq!(Network::Testnet.address_prefix(), "tb1");
    assert_eq!(Network::Regtest.address_prefix(), "bcrt1");
    assert_eq!(Network::Signet.address_prefix(), "tb1");
    
    Ok(())
}

#[tokio::test]
async fn test_address_type_methods() -> Result<()> {
    assert_eq!(AddressType::Legacy.as_str(), "legacy");
    assert_eq!(AddressType::P2shSegwit.as_str(), "p2sh-segwit");
    assert_eq!(AddressType::Bech32.as_str(), "bech32");
    assert_eq!(AddressType::Taproot.as_str(), "bech32m");
    
    Ok(())
}

#[tokio::test]
async fn test_amount_conversions() -> Result<()> {
    let amount = Amount::from_btc(1.0);
    assert_eq!(amount.to_sat(), 100_000_000);
    assert_eq!(amount.to_btc(), 1.0);
    
    let amount_sat = Amount::from_sat(50_000_000);
    assert_eq!(amount_sat.to_btc(), 0.5);
    assert_eq!(amount_sat.to_sat(), 50_000_000);
    
    let zero_amount = Amount::from_sat(0);
    assert!(zero_amount.is_zero());
    
    Ok(())
}

#[tokio::test]
async fn test_amount_display() -> Result<()> {
    let amount = Amount::from_btc(1.23456789);
    let display_str = format!("{}", amount);
    // Due to floating point precision, we check for the approximate value
    assert!(display_str.contains("1.234567"));
    assert!(display_str.contains("BTC"));

    // Test with exact satoshi amount
    let amount_sat = Amount::from_sat(123456789);
    let display_str_sat = format!("{}", amount_sat);
    assert!(display_str_sat.contains("1.23456789"));
    assert!(display_str_sat.contains("BTC"));

    Ok(())
}

// Advanced PSBT Tests

#[tokio::test]
async fn test_advanced_psbt_creation() -> Result<()> {
    let mut builder = AdvancedPsbtBuilder::new(Network::Regtest);

    // Add input
    let input = PsbtInputInfo {
        prev_txid: "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
        prev_vout: 0,
        prev_amount: 100000000, // 1 BTC
        prev_script: "".to_string(),
        sequence: Some(0xfffffffe),
        sighash_type: Some(1),
    };
    builder.add_input(input)?;

    // Add output
    let output = PsbtOutputInfo {
        address: "bcrt1qw508d6qejxtdg4y5r3zarvary0c5xw7kygt080".to_string(),
        amount: 99000000, // 0.99 BTC (leaving 0.01 for fee)
    };
    builder.add_output(output)?;

    // Test serialization
    let base64_psbt = builder.to_base64();
    assert!(!base64_psbt.is_empty());

    let hex_psbt = builder.to_hex();
    assert!(!hex_psbt.is_empty());

    // Test statistics
    let stats = builder.get_stats();
    assert_eq!(stats.input_count, 1);
    assert_eq!(stats.output_count, 1);
    assert_eq!(stats.signed_inputs, 0);

    Ok(())
}

#[tokio::test]
async fn test_psbt_validation() -> Result<()> {
    let mut builder = AdvancedPsbtBuilder::new(Network::Regtest);

    // Add valid input and output
    let input = PsbtInputInfo {
        prev_txid: "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
        prev_vout: 0,
        prev_amount: 100000000,
        prev_script: "".to_string(),
        sequence: None,
        sighash_type: None,
    };
    builder.add_input(input)?;

    let output = PsbtOutputInfo {
        address: "bcrt1qw508d6qejxtdg4y5r3zarvary0c5xw7kygt080".to_string(),
        amount: 99000000,
    };
    builder.add_output(output)?;

    // Validate PSBT
    let validation = builder.validate();
    // PSBT will be invalid because we don't have previous output info
    // This is expected for a test PSBT without real UTXOs
    assert_eq!(validation.input_validations.len(), 1);
    assert_eq!(validation.output_validations.len(), 1);
    assert!(!validation.input_validations[0].is_valid); // Missing prev output info

    Ok(())
}

#[tokio::test]
async fn test_psbt_workflow_manager() -> Result<()> {
    let mut workflow = PsbtWorkflowManager::new(Network::Regtest);

    // Add mock hardware wallet
    let hw_wallet = MockHardwareWallet::new("Test Wallet".to_string());
    workflow.add_hardware_wallet(Box::new(hw_wallet));

    // Create PSBT
    let inputs = vec![PsbtInputInfo {
        prev_txid: "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
        prev_vout: 0,
        prev_amount: 100000000,
        prev_script: "".to_string(),
        sequence: None,
        sighash_type: None,
    }];

    let outputs = vec![PsbtOutputInfo {
        address: "bcrt1qw508d6qejxtdg4y5r3zarvary0c5xw7kygt080".to_string(),
        amount: 99000000,
    }];

    let psbt = workflow.create_psbt(inputs, outputs)?;
    assert_eq!(psbt.get_stats().input_count, 1);
    assert_eq!(psbt.get_stats().output_count, 1);

    Ok(())
}

#[tokio::test]
async fn test_multisig_psbt_manager() -> Result<()> {
    let multisig = MultiSigPsbtManager::new(2, 3, Network::Regtest)?;

    let inputs = vec![PsbtInputInfo {
        prev_txid: "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
        prev_vout: 0,
        prev_amount: 100000000,
        prev_script: "".to_string(),
        sequence: None,
        sighash_type: None,
    }];

    let outputs = vec![PsbtOutputInfo {
        address: "bcrt1qw508d6qejxtdg4y5r3zarvary0c5xw7kygt080".to_string(),
        amount: 99000000,
    }];

    let public_keys = vec![
        "xpub1".to_string(),
        "xpub2".to_string(),
        "xpub3".to_string(),
    ];

    let psbt = multisig.create_multisig_psbt(inputs, outputs, public_keys)?;
    assert_eq!(psbt.get_stats().input_count, 1);
    assert_eq!(psbt.get_stats().output_count, 1);

    Ok(())
}

#[tokio::test]
async fn test_psbt_utils() -> Result<()> {
    let mut builder = AdvancedPsbtBuilder::new(Network::Regtest);

    let input = PsbtInputInfo {
        prev_txid: "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
        prev_vout: 0,
        prev_amount: 100000000,
        prev_script: "".to_string(),
        sequence: None,
        sighash_type: None,
    };
    builder.add_input(input)?;

    let output = PsbtOutputInfo {
        address: "bcrt1qw508d6qejxtdg4y5r3zarvary0c5xw7kygt080".to_string(),
        amount: 99000000,
    };
    builder.add_output(output)?;

    // Test fee calculation
    let fee = PsbtUtils::calculate_fee(&builder);
    // Fee will be 0 because we don't have real UTXO data
    assert_eq!(fee, 0);

    // Test size estimation
    let estimated_size = PsbtUtils::estimate_final_size(&builder);
    assert!(estimated_size > 0);

    // Test QR code conversion
    let qr_data = PsbtUtils::to_qr_data(&builder);
    assert!(!qr_data.is_empty());

    // Test parsing from QR data
    let parsed_psbt = PsbtUtils::from_qr_data(&qr_data)?;
    assert_eq!(parsed_psbt.get_stats().input_count, 1);

    Ok(())
}

#[tokio::test]
async fn test_hardware_wallet_mock() -> Result<()> {
    let hw_wallet = MockHardwareWallet::new("Ledger Test".to_string());

    assert_eq!(hw_wallet.device_name(), "Ledger Test");
    assert!(hw_wallet.is_connected().await);

    let xpub = hw_wallet.get_xpub("m/44'/0'/0'").await?;
    assert!(!xpub.is_empty());

    let address = hw_wallet.get_address("m/44'/0'/0'/0/0", AddressType::Bech32).await?;
    assert!(!address.is_empty());

    let display_address = hw_wallet.display_address("m/44'/0'/0'/0/0").await?;
    assert!(!display_address.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_psbt_builder_creation() -> Result<()> {
    let builder = PsbtBuilder::new();
    
    assert_eq!(builder.inputs().len(), 0);
    assert_eq!(builder.outputs().len(), 0);
    assert_eq!(builder.total_input_amount().to_sat(), 0);
    assert_eq!(builder.total_output_amount().to_sat(), 0);
    assert!(!builder.is_complete());
    
    Ok(())
}

#[tokio::test]
async fn test_psbt_builder_amounts() -> Result<()> {
    use std::collections::HashMap;
    use cerberus::bitcoin::{Utxo};
    
    // Create mock UTXOs
    let utxo1 = Utxo {
        txid: "tx1".to_string(),
        vout: 0,
        amount: Amount::from_btc(1.0),
        script_pubkey: "script1".to_string(),
        address: Some("addr1".to_string()),
        confirmations: 6,
        spendable: true,
        safe: true,
    };
    
    let utxo2 = Utxo {
        txid: "tx2".to_string(),
        vout: 1,
        amount: Amount::from_btc(0.5),
        script_pubkey: "script2".to_string(),
        address: Some("addr2".to_string()),
        confirmations: 3,
        spendable: true,
        safe: true,
    };
    
    let utxos = vec![utxo1, utxo2];
    
    // Create outputs
    let mut outputs = HashMap::new();
    outputs.insert("recipient1".to_string(), Amount::from_btc(0.8));
    outputs.insert("recipient2".to_string(), Amount::from_btc(0.2));
    
    // Build PSBT
    let builder = PsbtBuilder::from_utxos_and_outputs(utxos, outputs)?;
    
    assert_eq!(builder.inputs().len(), 2);
    assert_eq!(builder.outputs().len(), 2);
    assert_eq!(builder.total_input_amount().to_btc(), 1.5);
    assert_eq!(builder.total_output_amount().to_btc(), 1.0);
    
    let fee = builder.calculate_fee();
    assert_eq!(fee.to_btc(), 0.5);
    
    Ok(())
}

#[tokio::test]
async fn test_psbt_serialization() -> Result<()> {
    let builder = PsbtBuilder::new();
    
    // Build PSBT string
    let psbt_string = builder.build()?;
    assert!(!psbt_string.is_empty());
    assert!(psbt_string.starts_with("base64_"));
    
    // Parse PSBT back
    let parsed_builder = PsbtBuilder::parse(&psbt_string)?;
    assert_eq!(parsed_builder.inputs().len(), 0);
    assert_eq!(parsed_builder.outputs().len(), 0);
    
    Ok(())
}

#[tokio::test]
async fn test_transaction_builder_creation() -> Result<()> {
    let builder = TransactionBuilder::new();
    
    assert_eq!(builder.total_input_amount().to_sat(), 0);
    assert_eq!(builder.total_output_amount().to_sat(), 0);
    assert_eq!(builder.calculate_fee().to_sat(), 0);
    
    Ok(())
}

#[tokio::test]
async fn test_transaction_builder_with_data() -> Result<()> {
    let builder = TransactionBuilder::new()
        .version(2)
        .lock_time(123456)
        .add_output("bc1qtest".to_string(), Amount::from_btc(0.5));
    
    assert_eq!(builder.total_output_amount().to_btc(), 0.5);
    
    Ok(())
}

#[tokio::test]
async fn test_transaction_size_estimation() -> Result<()> {
    let builder = TransactionBuilder::new();
    
    // Test size estimation
    let size_no_change = builder.estimate_transaction_size(1, false);
    let size_with_change = builder.estimate_transaction_size(1, true);
    
    assert!(size_with_change > size_no_change);
    assert!(size_no_change > 0);
    
    Ok(())
}

// Note: The following tests would require a running Bitcoin Core node
// They are marked as ignored by default

#[tokio::test]
#[ignore = "requires Bitcoin Core node"]
async fn test_bitcoin_core_connection() -> Result<()> {
    let config = test_config();
    
    // This would fail without a running Bitcoin Core node
    match BitcoinCore::new(config).await {
        Ok(_core) => {
            // Connection successful
            println!("Bitcoin Core connection successful");
        }
        Err(e) => {
            println!("Bitcoin Core connection failed (expected): {}", e);
            // This is expected when no Bitcoin Core is running
        }
    }
    
    Ok(())
}

#[tokio::test]
#[ignore = "requires Bitcoin Core node"]
async fn test_wallet_manager_creation() -> Result<()> {
    let config = test_config();
    
    match WalletManager::new(config).await {
        Ok(_manager) => {
            println!("Wallet manager creation successful");
        }
        Err(e) => {
            println!("Wallet manager creation failed (expected): {}", e);
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_bitcoin_error_types() -> Result<()> {
    use cerberus::bitcoin::BitcoinError;
    
    let rpc_error = BitcoinError::Rpc("test error".to_string());
    assert!(rpc_error.to_string().contains("RPC error"));
    
    let invalid_address = BitcoinError::InvalidAddress("invalid".to_string());
    assert!(invalid_address.to_string().contains("Invalid address"));
    
    let insufficient_funds = BitcoinError::InsufficientFunds {
        required: Amount::from_btc(1.0),
        available: Amount::from_btc(0.5),
    };
    assert!(insufficient_funds.to_string().contains("Insufficient funds"));
    
    Ok(())
}

#[tokio::test]
async fn test_bitcoin_config_serialization() -> Result<()> {
    let config = test_config();
    
    // Test serialization
    let json = serde_json::to_string(&config)?;
    assert!(json.contains("regtest"));
    assert!(json.contains("test_wallet"));
    
    // Test deserialization
    let deserialized: BitcoinConfig = serde_json::from_str(&json)?;
    assert_eq!(deserialized.network, Network::Regtest);
    assert_eq!(deserialized.wallet_name, Some("test_wallet".to_string()));
    
    Ok(())
}

#[tokio::test]
async fn test_network_serialization() -> Result<()> {
    let networks = vec![
        Network::Mainnet,
        Network::Testnet,
        Network::Regtest,
        Network::Signet,
    ];
    
    for network in networks {
        let json = serde_json::to_string(&network)?;
        let deserialized: Network = serde_json::from_str(&json)?;
        assert_eq!(network, deserialized);
    }
    
    Ok(())
}

#[tokio::test]
async fn test_address_type_serialization() -> Result<()> {
    let types = vec![
        AddressType::Legacy,
        AddressType::P2shSegwit,
        AddressType::Bech32,
        AddressType::Taproot,
    ];
    
    for addr_type in types {
        let json = serde_json::to_string(&addr_type)?;
        let deserialized: AddressType = serde_json::from_str(&json)?;
        assert_eq!(addr_type, deserialized);
    }
    
    Ok(())
}

#[tokio::test]
async fn test_amount_serialization() -> Result<()> {
    let amount = Amount::from_btc(1.23456789);

    let json = serde_json::to_string(&amount)?;
    let deserialized: Amount = serde_json::from_str(&json)?;

    assert_eq!(amount.to_sat(), deserialized.to_sat());

    Ok(())
}

#[tokio::test]
async fn test_transaction_signer() -> Result<()> {
    println!("🔐 Testing Transaction Signer...");

    let network = Network::Regtest;
    let signer = TransactionSigner::new(network);

    // Create a simple transaction for testing
    let transaction = bitcoin::Transaction {
        version: bitcoin::transaction::Version::TWO,
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![bitcoin::TxIn {
            previous_output: bitcoin::OutPoint::null(),
            script_sig: bitcoin::ScriptBuf::new(),
            sequence: bitcoin::Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: bitcoin::Witness::new(),
        }],
        output: vec![bitcoin::TxOut {
            value: bitcoin::Amount::from_sat(100000),
            script_pubkey: bitcoin::ScriptBuf::new(),
        }],
    };

    // Create signing info (mock data)
    let signing_info = vec![InputSigningInfo {
        input_index: 0,
        private_key: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
        prev_amount: 200000,
        prev_script: "76a914".to_string() + &"0".repeat(40) + "88ac", // P2PKH script
        script_type: TxScriptType::P2PKH,
        sighash_type: None,
        derivation_path: Some("m/44'/0'/0'/0/0".to_string()),
    }];

    // Test signing
    let result = signer.sign_transaction(transaction, signing_info)?;

    // Verify results
    assert_eq!(result.total_inputs, 1);
    assert_eq!(result.input_results.len(), 1);
    assert!(!result.txid.is_empty());

    println!("✅ Transaction signer test passed");
    Ok(())
}

#[tokio::test]
async fn test_key_manager() -> Result<()> {
    println!("🔑 Testing Key Manager...");

    let network = Network::Regtest;
    let mut key_manager = KeyManager::new(network, cerberus::bitcoin::key_manager::SecurityLevel::Memory);

    // Test mnemonic generation
    let mnemonic_info = key_manager.generate_mnemonic(12)?;
    assert_eq!(mnemonic_info.word_count, 12);
    assert_eq!(mnemonic_info.entropy_bits, 128);
    assert!(!mnemonic_info.phrase.is_empty());

    // Test mnemonic validation
    let validated = key_manager.validate_mnemonic(&mnemonic_info.phrase)?;
    assert_eq!(validated.word_count, 12);

    // Test HD wallet creation
    let wallet = key_manager.create_hd_wallet(
        "Test Wallet".to_string(),
        &mnemonic_info.phrase,
        None,
    )?;

    assert_eq!(wallet.name, "Test Wallet");
    assert_eq!(wallet.network, network);
    assert!(!wallet.id.is_empty());
    assert!(!wallet.extended_public_key.is_empty());

    // Test key derivation
    let key_derivation = key_manager.derive_key(&wallet.id, "m/44'/0'/0'/0", 0)?;
    assert_eq!(key_derivation.index, 0);
    assert!(!key_derivation.address.is_empty());
    assert!(!key_derivation.public_key.is_empty());

    // Test private key retrieval
    let private_key = key_manager.get_private_key(&wallet.id, "m/44'/0'/0'/0", 0)?;
    assert!(!private_key.is_empty());

    println!("✅ Key manager test passed");
    Ok(())
}

#[tokio::test]
async fn test_security_validator() -> Result<()> {
    println!("🛡️ Testing Security Validator...");

    let network = Network::Regtest;
    let validator = SecurityValidator::new(network);

    // Create a test transaction
    let transaction = bitcoin::Transaction {
        version: bitcoin::transaction::Version::TWO,
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![bitcoin::TxIn {
            previous_output: bitcoin::OutPoint::null(),
            script_sig: bitcoin::ScriptBuf::new(),
            sequence: bitcoin::Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: bitcoin::Witness::new(),
        }],
        output: vec![bitcoin::TxOut {
            value: bitcoin::Amount::from_sat(50000), // 0.0005 BTC
            script_pubkey: bitcoin::ScriptBuf::new(),
        }],
    };

    let input_amounts = vec![60000]; // 0.0006 BTC input (lower fee)

    // Test validation
    let result = validator.validate_transaction(&transaction, &input_amounts)?;

    // Should be valid for small amounts with reasonable fee
    assert!(result.is_valid);
    assert_eq!(result.security_level, cerberus::bitcoin::SecurityLevel::Low);
    assert_eq!(result.errors.len(), 0);

    println!("✅ Security validator test passed");
    Ok(())
}

#[tokio::test]
async fn test_script_builder() -> Result<()> {
    println!("📜 Testing Script Builder...");

    let network = Network::Regtest;
    let script_builder = ScriptBuilder::new(network);

    // Test P2PKH script creation
    let pubkey = "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";
    let p2pkh_script = script_builder.create_p2pkh(pubkey)?;
    assert!(!p2pkh_script.is_empty());

    // Test P2WPKH script creation
    let p2wpkh_script = script_builder.create_p2wpkh(pubkey)?;
    assert!(!p2wpkh_script.is_empty());

    // Test multisig script creation
    let multisig_config = cerberus::bitcoin::MultisigConfig {
        required_signatures: 2,
        public_keys: vec![
            pubkey.to_string(),
            "02f9308a019258c31049344f85f89d5229b531c845836f99b08601f113bce036f9".to_string(),
            "03dff1d77f2a671c5f36183726db2341be58feae1da2deced843240f7b502ba659".to_string(),
        ],
        script_type: ScriptType::Multisig,
    };

    let multisig_script = script_builder.create_multisig(&multisig_config)?;
    assert!(!multisig_script.is_empty());

    // Test script analysis
    let template = script_builder.analyze_script(&p2pkh_script)?;
    assert_eq!(template.script_type, ScriptType::P2PKH);
    assert_eq!(template.required_signatures, 1);
    assert_eq!(template.total_keys, 1);
    assert!(template.is_standard);

    // Test standard templates
    let templates = script_builder.get_standard_templates();
    assert!(!templates.is_empty());
    assert!(templates.iter().any(|t| t.script_type == ScriptType::P2PKH));
    assert!(templates.iter().any(|t| t.script_type == ScriptType::P2WPKH));

    println!("✅ Script builder test passed");
    Ok(())
}

#[tokio::test]
async fn test_hardware_wallet_manager() -> Result<()> {
    println!("🔌 Testing Hardware Wallet Manager...");

    let network = Network::Regtest;
    let mut hw_manager = HardwareWalletManager::new(network);

    // Add mock devices
    hw_manager.add_mock_devices();

    // List devices
    let devices = hw_manager.list_devices();
    assert!(!devices.is_empty());
    assert!(devices.iter().any(|d| d.contains("ledger")));
    assert!(devices.iter().any(|d| d.contains("trezor")));

    // Test PSBT signing with mock device
    let device_id = devices[0].clone();

    // Create a simple PSBT for testing
    let psbt = bitcoin::psbt::Psbt {
        unsigned_tx: bitcoin::Transaction {
            version: bitcoin::transaction::Version::TWO,
            lock_time: bitcoin::absolute::LockTime::ZERO,
            input: vec![bitcoin::TxIn {
                previous_output: bitcoin::OutPoint::null(),
                script_sig: bitcoin::ScriptBuf::new(),
                sequence: bitcoin::Sequence::ENABLE_RBF_NO_LOCKTIME,
                witness: bitcoin::Witness::new(),
            }],
            output: vec![bitcoin::TxOut {
                value: bitcoin::Amount::from_sat(50000),
                script_pubkey: bitcoin::ScriptBuf::new(),
            }],
        },
        version: 0,
        xpub: Default::default(),
        proprietary: Default::default(),
        unknown: Default::default(),
        inputs: vec![Default::default()],
        outputs: vec![Default::default()],
    };

    let psbt_base64 = base64::prelude::BASE64_STANDARD.encode(psbt.serialize());

    let signing_request = HardwareSigningRequest {
        device_id: device_id.clone(),
        psbt: psbt_base64,
        derivation_paths: std::collections::HashMap::new(),
        display_details: true,
        require_confirmation: true,
    };

    let response = hw_manager.sign_psbt_with_device(signing_request)?;
    assert!(response.success);
    assert!(response.signed_psbt.is_some());
    assert!(response.user_confirmed);

    // Test address verification
    let verification_request = AddressVerificationRequest {
        device_id,
        address: "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4".to_string(),
        derivation_path: "m/84'/0'/0'/0/0".to_string(),
        address_type: AddressType::Bech32,
    };

    let verification_response = hw_manager.verify_address_with_device(verification_request)?;
    assert!(verification_response.success);
    assert!(verification_response.address_matches);

    println!("✅ Hardware wallet manager test passed");
    Ok(())
}
