#![cfg(test)]

use settlement_registry::types::CaseStatus;
use settlement_registry::{SettlementRegistry, SettlementRegistryClient};
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{Address, BytesN, Env};

const EXPECTED_RELEASE_CHECKSUM: &str =
    "1018a81b1ac95046cb00466ceda7ee347204c08b71b1c51b3c9611dd32215d66";

fn sample_bytes(env: &Env, val: u8) -> BytesN<32> {
    let raw = [val; 32];
    BytesN::from_array(env, &raw)
}

fn find_file_path(relative_path: &str) -> Option<std::path::PathBuf> {
    let direct = std::path::PathBuf::from(relative_path);
    if direct.exists() {
        return Some(direct);
    }
    let from_crate = std::path::PathBuf::from("../../").join(relative_path);
    if from_crate.exists() {
        return Some(from_crate);
    }
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let from_manifest = std::path::PathBuf::from(manifest_dir)
            .join("../../")
            .join(relative_path);
        if from_manifest.exists() {
            return Some(from_manifest);
        }
    }
    None
}

#[test]
fn test_release_chain_source_revision_and_version() {
    let candidates = [
        std::path::PathBuf::from("Cargo.toml"),
        std::path::PathBuf::from("../../Cargo.toml"),
    ];
    let mut root_cargo = None;
    for c in candidates {
        if let Ok(content) = std::fs::read_to_string(&c) {
            if content.contains("[workspace]") {
                root_cargo = Some(content);
                break;
            }
        }
    }
    let content = root_cargo.expect("Root Cargo.toml must exist");

    assert!(
        content.contains("version = \"0.1.0\""),
        "Workspace version must match release candidate version 0.1.0"
    );
}

#[test]
fn test_release_artifact_wasm_binary_integrity() {
    let env = Env::default();
    let wasm_path = find_file_path("artifacts/settlement_registry.wasm")
        .or_else(|| find_file_path("target/wasm32v1-none/release/settlement_registry.wasm"))
        .expect("Published or compiled WASM artifact must be present");
    let wasm_bytes = std::fs::read(wasm_path).expect("Must read WASM bytes");

    // 1. Validate WASM magic bytes and format
    assert!(
        wasm_bytes.len() > 4,
        "WASM binary must be greater than header size"
    );
    assert_eq!(
        &wasm_bytes[0..4],
        b"\0asm",
        "WASM artifact must start with standard WebAssembly magic bytes"
    );

    // 2. Validate SHA-256 calculation
    let wasm_sdk_bytes = soroban_sdk::Bytes::from_slice(&env, &wasm_bytes);
    let calculated_hash = env.crypto().sha256(&wasm_sdk_bytes);
    assert_eq!(calculated_hash.to_array().len(), 32);

    let mut calculated_hex = String::new();
    for byte in calculated_hash.to_array() {
        calculated_hex.push_str(&format!("{:02x}", byte));
    }
    assert_eq!(
        calculated_hex, EXPECTED_RELEASE_CHECKSUM,
        "Compiled WASM hash must match canonical v0.1.0 release checksum"
    );

    // 3. Verify match with checksum file if present
    if let Some(sha_path) = find_file_path("artifacts/settlement_registry.wasm.sha256") {
        if let Ok(sha_content) = std::fs::read_to_string(sha_path) {
            let published_hash_hex = sha_content
                .split_whitespace()
                .next()
                .expect("SHA256 checksum file must contain hash string");
            assert_eq!(
                calculated_hex, published_hash_hex,
                "Calculated artifact hash must match published checksum file"
            );
        }
    }
}

#[test]
fn test_release_manifest_metadata_consistency() {
    if let Some(manifest_path) = find_file_path("artifacts/release-manifest.json") {
        let manifest_content = std::fs::read_to_string(manifest_path).unwrap();
        assert!(manifest_content.contains("\"contract_name\": \"settlement_registry\""));
        assert!(manifest_content.contains("\"version\": \"0.1.0\""));
        assert!(manifest_content.contains("\"wasm_file\": \"settlement_registry.wasm\""));
        assert!(manifest_content.contains(EXPECTED_RELEASE_CHECKSUM));
        assert!(manifest_content.contains("\"target\": \"wasm32v1-none\""));
        assert!(manifest_content.contains("\"networks\""));
        assert!(manifest_content.contains("\"testnet\""));
        assert!(manifest_content.contains("\"mainnet\""));
        assert!(manifest_content.contains("https://soroban-testnet.stellar.org"));
        assert!(manifest_content.contains("Test SDF Network ; September 2015"));
        assert!(manifest_content.contains("Public Global Stellar Network ; July 2015"));
    }
}

#[test]
fn test_versioned_archive_consistency() {
    if let (Some(versioned_path), Some(active_path)) = (
        find_file_path("artifacts/v0.1.0/settlement_registry.wasm"),
        find_file_path("artifacts/settlement_registry.wasm"),
    ) {
        let v_bytes = std::fs::read(versioned_path).unwrap();
        let a_bytes = std::fs::read(active_path).unwrap();
        assert_eq!(
            v_bytes, a_bytes,
            "Versioned v0.1.0 artifact must be identical to active artifact"
        );
    }
    if let (Some(versioned_sha), Some(active_sha)) = (
        find_file_path("artifacts/v0.1.0/settlement_registry.wasm.sha256"),
        find_file_path("artifacts/settlement_registry.wasm.sha256"),
    ) {
        let v_sha = std::fs::read_to_string(versioned_sha).unwrap();
        let a_sha = std::fs::read_to_string(active_sha).unwrap();
        assert_eq!(
            v_sha.trim(),
            a_sha.trim(),
            "Versioned v0.1.0 SHA-256 must match active checksum file"
        );
    }
}

#[test]
fn test_release_contract_execution_in_soroban_env() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_sequence_number(1_000);

    let admin = Address::generate(&env);
    let contract_id = env.register(SettlementRegistry, (&admin,));
    let client = SettlementRegistryClient::new(&env, &contract_id);

    let observer = Address::generate(&env);
    client.add_observer(&observer);
    assert!(client.is_observer(&observer));

    let owner = Address::generate(&env);
    let case_id = sample_bytes(&env, 123);
    let terms = sample_bytes(&env, 234);

    client.create_case(&case_id, &owner, &None, &terms, &5_000);
    let case_data = client.get_case(&case_id);
    assert_eq!(case_data.owner, owner);
    assert_eq!(case_data.status, CaseStatus::Open);
}

#[test]
fn test_released_deployment_identity_and_constructor_boundary() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_sequence_number(1_000);

    let admin = Address::generate(&env);
    let contract_id = env.register(SettlementRegistry, (&admin,));
    let client = SettlementRegistryClient::new(&env, &contract_id);

    // Initial state check: no observer registered yet
    let random_observer = Address::generate(&env);
    assert!(!client.is_observer(&random_observer));

    // Admin can register observer
    client.add_observer(&random_observer);
    assert!(client.is_observer(&random_observer));

    // Revocation check
    client.remove_observer(&random_observer);
    assert!(!client.is_observer(&random_observer));
}

#[test]
fn test_release_chain_mismatch_failure_conditions() {
    let env = Env::default();
    let sample_data = b"tampered_wasm_payload";
    let sdk_bytes = soroban_sdk::Bytes::from_slice(&env, sample_data);
    let hash = env.crypto().sha256(&sdk_bytes);

    let mut calculated_hex = String::new();
    for byte in hash.to_array() {
        calculated_hex.push_str(&format!("{:02x}", byte));
    }

    // Tampered payload hash must NOT match expected release checksum
    assert_ne!(
        calculated_hex, EXPECTED_RELEASE_CHECKSUM,
        "Tampered artifact hash must fail verification against published checksum"
    );
}
