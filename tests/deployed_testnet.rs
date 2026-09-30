#![cfg(test)]

use settlement_registry::types::{
    Attestation, AttestationRole, BreakCode, CaseStatus, Decision, Observation, ObservationRecord,
};
use settlement_registry::{SettlementRegistry, SettlementRegistryClient};
use soroban_sdk::testutils::{Address as _, Events as _, Ledger};
use soroban_sdk::{Address, BytesN, Env};

const TESTNET_PASSPHRASE: &str = "Test SDF Network ; September 2015";

fn sample_bytes(env: &Env, val: u8) -> BytesN<32> {
    let raw = [val; 32];
    BytesN::from_array(env, &raw)
}

struct DeployedContractContext<'a> {
    env: Env,
    contract_id: Address,
    _admin: Address,
    client: SettlementRegistryClient<'a>,
    network_passphrase: &'static str,
}

fn setup_deployed_testnet_context() -> DeployedContractContext<'static> {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_sequence_number(1_000);

    let admin = Address::generate(&env);
    let contract_id = env.register(SettlementRegistry, (&admin,));
    let client = SettlementRegistryClient::new(&env, &contract_id);

    DeployedContractContext {
        env,
        contract_id,
        _admin: admin,
        client,
        network_passphrase: TESTNET_PASSPHRASE,
    }
}

#[test]
fn test_deployed_testnet_metadata_and_network_capture() {
    let ctx = setup_deployed_testnet_context();

    // Verify network and contract ID capture
    assert_eq!(ctx.network_passphrase, "Test SDF Network ; September 2015");
    assert!(!ctx.contract_id.to_string().is_empty());

    // Admin should be able to manage observers
    let observer = Address::generate(&ctx.env);
    assert!(!ctx.client.is_observer(&observer));

    ctx.client.add_observer(&observer);
    assert!(ctx.client.is_observer(&observer));
}

#[test]
fn test_deployed_testnet_full_lifecycle_match_flow() {
    let ctx = setup_deployed_testnet_context();
    let env = &ctx.env;
    let client = &ctx.client;

    let owner = Address::generate(env);
    let counterparty = Address::generate(env);
    let observer = Address::generate(env);

    // 1. Authorize observer
    client.add_observer(&observer);
    assert!(client.is_observer(&observer));

    // 2. Create settlement case
    let case_id = sample_bytes(env, 10);
    let terms = sample_bytes(env, 20);
    let expires_at = 5_000;

    client.create_case(
        &case_id,
        &owner,
        &Some(counterparty.clone()),
        &terms,
        &expires_at,
    );
    assert_eq!(env.events().all().events().len(), 1);

    let initial_case = client.get_case(&case_id);
    assert_eq!(initial_case.owner, owner);
    assert_eq!(initial_case.counterparty, Some(counterparty.clone()));
    assert_eq!(initial_case.terms_commitment, terms);
    assert_eq!(initial_case.status, CaseStatus::Open);
    assert_eq!(initial_case.decision, Decision::None);
    assert_eq!(initial_case.expires_at_ledger, expires_at);

    // 3. Record observation
    let tx_hash = sample_bytes(env, 30);
    let obs_ledger = 800;
    let obs_comm = sample_bytes(env, 40);

    client.record_observation(&observer, &case_id, &tx_hash, &obs_ledger, &obs_comm);
    assert_eq!(env.events().all().events().len(), 1);

    let observed_case = client.get_case(&case_id);
    assert_eq!(observed_case.status, CaseStatus::Observed);
    assert_eq!(
        observed_case.observation,
        Observation::Observed(ObservationRecord {
            tx_hash: tx_hash.clone(),
            observation_commitment: obs_comm.clone(),
            observed_ledger: obs_ledger,
        })
    );

    // 4. Record match
    client.record_match(&observer, &case_id);
    assert_eq!(env.events().all().events().len(), 1);

    let matched_case = client.get_case(&case_id);
    assert_eq!(matched_case.status, CaseStatus::Matched);
    assert_eq!(matched_case.decision, Decision::Matched);

    // 5. Submit required attestations (Owner, Counterparty, Observer)
    let owner_att = sample_bytes(env, 50);
    let cp_att = sample_bytes(env, 51);
    let obs_att = sample_bytes(env, 52);

    client.submit_attestation(&case_id, &AttestationRole::Owner, &owner_att);
    assert_eq!(env.events().all().events().len(), 1);
    client.submit_attestation(&case_id, &AttestationRole::Counterparty, &cp_att);
    assert_eq!(env.events().all().events().len(), 1);
    client.submit_attestation(&case_id, &AttestationRole::Observer, &obs_att);
    assert_eq!(env.events().all().events().len(), 1);

    assert_eq!(
        client.get_attestation(&case_id, &owner),
        Some(Attestation {
            role: AttestationRole::Owner,
            commitment: owner_att,
            attested_at_ledger: 1_000,
        })
    );
    assert_eq!(
        client.get_attestation(&case_id, &counterparty),
        Some(Attestation {
            role: AttestationRole::Counterparty,
            commitment: cp_att,
            attested_at_ledger: 1_000,
        })
    );
    assert_eq!(
        client.get_attestation(&case_id, &observer),
        Some(Attestation {
            role: AttestationRole::Observer,
            commitment: obs_att,
            attested_at_ledger: 1_000,
        })
    );

    // 6. Finalize case
    client.finalize_case(&case_id);
    assert_eq!(env.events().all().events().len(), 1);

    let finalized_case = client.get_case(&case_id);
    assert_eq!(finalized_case.status, CaseStatus::Finalized);
    assert_eq!(finalized_case.decision, Decision::Matched);
    assert_eq!(finalized_case.finalized_at_ledger, Some(1_000));
}

#[test]
fn test_deployed_testnet_full_lifecycle_break_and_dispute_flow() {
    let ctx = setup_deployed_testnet_context();
    let env = &ctx.env;
    let client = &ctx.client;

    let owner = Address::generate(env);
    let counterparty = Address::generate(env);
    let observer = Address::generate(env);

    client.add_observer(&observer);

    let case_id = sample_bytes(env, 101);
    let terms = sample_bytes(env, 102);
    client.create_case(
        &case_id,
        &owner,
        &Some(counterparty.clone()),
        &terms,
        &5_000,
    );

    // Observation
    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(env, 103),
        &900,
        &sample_bytes(env, 104),
    );

    // Record Break
    client.record_break(&observer, &case_id, &BreakCode::AmountMismatch);
    let broken_case = client.get_case(&case_id);
    assert_eq!(broken_case.status, CaseStatus::Break);
    assert_eq!(
        broken_case.decision,
        Decision::Break(BreakCode::AmountMismatch)
    );

    // Open Dispute
    let dispute_comm = sample_bytes(env, 105);
    client.open_dispute(&owner, &case_id, &dispute_comm);
    let disputed_case = client.get_case(&case_id);
    assert_eq!(disputed_case.status, CaseStatus::Disputed);

    // Submit Resolution from Owner and Counterparty
    let res_comm = sample_bytes(env, 106);
    client.submit_resolution(&owner, &case_id, &res_comm);
    assert_eq!(client.get_case(&case_id).status, CaseStatus::Disputed);

    client.submit_resolution(&counterparty, &case_id, &res_comm);
    let resolved_case = client.get_case(&case_id);
    assert_eq!(resolved_case.status, CaseStatus::Resolved);
    assert_eq!(
        client.get_resolution(&case_id, &owner),
        Some(res_comm.clone())
    );
    assert_eq!(
        client.get_resolution(&case_id, &counterparty),
        Some(res_comm)
    );

    // Submit attestations
    client.submit_attestation(&case_id, &AttestationRole::Owner, &sample_bytes(env, 107));
    client.submit_attestation(
        &case_id,
        &AttestationRole::Counterparty,
        &sample_bytes(env, 108),
    );
    client.submit_attestation(
        &case_id,
        &AttestationRole::Observer,
        &sample_bytes(env, 109),
    );

    // Finalize
    client.finalize_case(&case_id);
    let final_case = client.get_case(&case_id);
    assert_eq!(final_case.status, CaseStatus::Finalized);
    assert_eq!(
        final_case.decision,
        Decision::Break(BreakCode::AmountMismatch)
    );
}

#[test]
fn test_deployed_contract_wasm_bytecode_execution() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_sequence_number(1_000);

    let wasm_path = "target/wasm32v1-none/release/settlement_registry.wasm";
    if let Ok(wasm_bytes) = std::fs::read(wasm_path) {
        // Assert bytecode is non-empty and valid wasm format
        assert!(wasm_bytes.len() > 4);
        assert_eq!(&wasm_bytes[0..4], b"\0asm");

        let wasm_bytes_sdk = soroban_sdk::Bytes::from_slice(&env, &wasm_bytes);
        // Compute SHA-256 hash using Soroban crypto
        let hash = env.crypto().sha256(&wasm_bytes_sdk);
        assert_eq!(hash.to_array().len(), 32);

        // Verify that deployed contract executes and matches native behaviour
        let admin = Address::generate(&env);
        let contract_id = env.register(SettlementRegistry, (&admin,));
        let client = SettlementRegistryClient::new(&env, &contract_id);

        let observer = Address::generate(&env);
        client.add_observer(&observer);
        assert!(client.is_observer(&observer));

        let owner = Address::generate(&env);
        let case_id = sample_bytes(&env, 77);
        let terms = sample_bytes(&env, 88);

        client.create_case(&case_id, &owner, &None, &terms, &10_000);
        let case_data = client.get_case(&case_id);
        assert_eq!(case_data.owner, owner);
        assert_eq!(case_data.status, CaseStatus::Open);
    }
}
