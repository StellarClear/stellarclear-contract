#![cfg(test)]

use crate::errors::Error;
use crate::storage::PROTOCOL_VERSION;
use crate::types::{
    AttestationRole, BreakCode, CaseStatus, Decision, Observation, ObservationRecord,
};
use crate::{SettlementRegistry, SettlementRegistryClient};
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{Address, BytesN, Env};

fn create_test_env() -> (Env, Address, SettlementRegistryClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();

    // Set initial ledger sequence
    env.ledger().set_sequence_number(100);

    let admin = Address::generate(&env);
    let contract_id = env.register(SettlementRegistry, (&admin,));
    let client = SettlementRegistryClient::new(&env, &contract_id);

    (env, admin, client)
}

fn sample_bytes(env: &Env, fill: u8) -> BytesN<32> {
    BytesN::from_array(env, &[fill; 32])
}

fn zero_bytes(env: &Env) -> BytesN<32> {
    BytesN::from_array(env, &[0u8; 32])
}

// ---------------------------------------------------------------------------
// INITIALIZATION TESTS
// ---------------------------------------------------------------------------

#[test]
fn test_constructor_initialization() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(SettlementRegistry, (&admin,));
    let _client = SettlementRegistryClient::new(&env, &contract_id);

    // Constructor ran automatically via register
    assert_eq!(PROTOCOL_VERSION, 1);
}

#[test]
fn test_cannot_reinitialize() {
    let (env, admin, _client) = create_test_env();
    let contract_id = env.register(SettlementRegistry, (&admin,));

    // Calling constructor again in contract context must fail with AlreadyInitialized
    let res = env.as_contract(&contract_id, || {
        SettlementRegistry::__constructor(env.clone(), admin.clone())
    });
    assert_eq!(res, Err(Error::AlreadyInitialized));
}

// ---------------------------------------------------------------------------
// OBSERVER REGISTRATION TESTS
// ---------------------------------------------------------------------------

#[test]
fn test_add_and_remove_observer() {
    let (env, _admin, client) = create_test_env();
    let observer = Address::generate(&env);

    assert!(!client.is_observer(&observer));

    // Add observer
    client.add_observer(&observer);
    assert!(client.is_observer(&observer));

    // Duplicate add should fail
    let res = client.try_add_observer(&observer);
    assert_eq!(res, Err(Ok(Error::ObserverAlreadyRegistered)));

    // Remove observer
    client.remove_observer(&observer);
    assert!(!client.is_observer(&observer));

    // Removing non-existent observer fails
    let res = client.try_remove_observer(&observer);
    assert_eq!(res, Err(Ok(Error::ObserverNotRegistered)));
}

// ---------------------------------------------------------------------------
// CASE CREATION TESTS
// ---------------------------------------------------------------------------

#[test]
fn test_create_case_success() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let counterparty = Address::generate(&env);
    let case_id = sample_bytes(&env, 1);
    let terms_commitment = sample_bytes(&env, 2);
    let expires_at_ledger = 200;

    client.create_case(
        &case_id,
        &owner,
        &Some(counterparty.clone()),
        &terms_commitment,
        &expires_at_ledger,
    );

    let case = client.get_case(&case_id);
    assert_eq!(case.owner, owner);
    assert_eq!(case.counterparty, Some(counterparty));
    assert_eq!(case.terms_commitment, terms_commitment);
    assert_eq!(case.expires_at_ledger, expires_at_ledger);
    assert_eq!(case.status, CaseStatus::Open);
    assert_eq!(case.created_at_ledger, 100);
    assert_eq!(case.observation, Observation::None);
    assert_eq!(case.decision, Decision::None);
    assert_eq!(case.finalized_at_ledger, None);
}

#[test]
fn test_create_case_without_counterparty() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let case_id = sample_bytes(&env, 1);
    let terms_commitment = sample_bytes(&env, 2);

    client.create_case(&case_id, &owner, &None, &terms_commitment, &150);

    let case = client.get_case(&case_id);
    assert_eq!(case.counterparty, None);
    assert_eq!(case.status, CaseStatus::Open);
}

#[test]
fn test_create_case_duplicate_fails() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let case_id = sample_bytes(&env, 1);
    let terms_commitment = sample_bytes(&env, 2);

    client.create_case(&case_id, &owner, &None, &terms_commitment, &150);

    let res = client.try_create_case(&case_id, &owner, &None, &terms_commitment, &150);
    assert_eq!(res, Err(Ok(Error::CaseAlreadyExists)));
}

#[test]
fn test_create_case_zero_commitment_fails() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let case_id = sample_bytes(&env, 1);
    let zero_commitment = zero_bytes(&env);

    let res = client.try_create_case(&case_id, &owner, &None, &zero_commitment, &150);
    assert_eq!(res, Err(Ok(Error::InvalidCommitment)));
}

#[test]
fn test_create_case_invalid_expiration_fails() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let case_id = sample_bytes(&env, 1);
    let terms_commitment = sample_bytes(&env, 2);

    // Current ledger is 100; expiry at 100 or earlier must fail
    let res = client.try_create_case(&case_id, &owner, &None, &terms_commitment, &100);
    assert_eq!(res, Err(Ok(Error::InvalidExpiration)));

    let res = client.try_create_case(&case_id, &owner, &None, &terms_commitment, &50);
    assert_eq!(res, Err(Ok(Error::InvalidExpiration)));
}

#[test]
fn test_create_case_owner_as_counterparty_fails() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let case_id = sample_bytes(&env, 1);
    let terms_commitment = sample_bytes(&env, 2);

    let res = client.try_create_case(
        &case_id,
        &owner,
        &Some(owner.clone()),
        &terms_commitment,
        &150,
    );
    assert_eq!(res, Err(Ok(Error::CounterpartyNotAllowed)));
}

// ---------------------------------------------------------------------------
// OBSERVATION RECORDING TESTS
// ---------------------------------------------------------------------------

#[test]
fn test_record_observation_success() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    let case_id = sample_bytes(&env, 1);
    client.create_case(&case_id, &owner, &None, &sample_bytes(&env, 2), &200);

    let tx_hash = sample_bytes(&env, 3);
    let obs_commitment = sample_bytes(&env, 4);
    client.record_observation(&observer, &case_id, &tx_hash, &100, &obs_commitment);

    let case = client.get_case(&case_id);
    assert_eq!(case.status, CaseStatus::Observed);
    assert_eq!(
        case.observation,
        Observation::Observed(ObservationRecord {
            tx_hash,
            observation_commitment: obs_commitment,
            observed_ledger: 100,
        })
    );
}

#[test]
fn test_record_observation_unregistered_observer_fails() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let unregistered = Address::generate(&env);

    let case_id = sample_bytes(&env, 1);
    client.create_case(&case_id, &owner, &None, &sample_bytes(&env, 2), &200);

    let res = client.try_record_observation(
        &unregistered,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );
    assert_eq!(res, Err(Ok(Error::ObserverNotRegistered)));
}

#[test]
fn test_record_observation_second_observation_fails() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    let case_id = sample_bytes(&env, 1);
    client.create_case(&case_id, &owner, &None, &sample_bytes(&env, 2), &200);
    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );

    // Second observation must fail
    let res = client.try_record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 5),
        &100,
        &sample_bytes(&env, 6),
    );
    assert_eq!(res, Err(Ok(Error::InvalidState)));
}

#[test]
fn test_record_observation_invalid_ledger_fails() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    let case_id = sample_bytes(&env, 1);
    client.create_case(&case_id, &owner, &None, &sample_bytes(&env, 2), &200);

    // Zero ledger sequence
    let res = client.try_record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &0,
        &sample_bytes(&env, 4),
    );
    assert_eq!(res, Err(Ok(Error::InvalidLedger)));

    // Future ledger sequence (> current ledger 100)
    let res = client.try_record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &105,
        &sample_bytes(&env, 4),
    );
    assert_eq!(res, Err(Ok(Error::InvalidLedger)));
}

#[test]
fn test_record_observation_zero_commitment_fails() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    let case_id = sample_bytes(&env, 1);
    client.create_case(&case_id, &owner, &None, &sample_bytes(&env, 2), &200);

    let res = client.try_record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &zero_bytes(&env),
    );
    assert_eq!(res, Err(Ok(Error::InvalidCommitment)));
}

// ---------------------------------------------------------------------------
// RECONCILIATION DECISION TESTS (MATCH & BREAK)
// ---------------------------------------------------------------------------

#[test]
fn test_record_match_success() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    let case_id = sample_bytes(&env, 1);
    client.create_case(&case_id, &owner, &None, &sample_bytes(&env, 2), &200);
    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );

    client.record_match(&observer, &case_id);

    let case = client.get_case(&case_id);
    assert_eq!(case.status, CaseStatus::Matched);
    assert_eq!(case.decision, Decision::Matched);
}

#[test]
fn test_record_break_all_break_codes() {
    let break_codes = [
        BreakCode::AmountMismatch,
        BreakCode::AssetMismatch,
        BreakCode::DestinationMismatch,
        BreakCode::ReferenceMismatch,
        BreakCode::MissingSettlement,
        BreakCode::DuplicateSettlement,
        BreakCode::LateSettlement,
        BreakCode::FailedTransaction,
        BreakCode::UnexpectedTransaction,
    ];

    for (i, &break_code) in break_codes.iter().enumerate() {
        let (env, _admin, client) = create_test_env();
        let owner = Address::generate(&env);
        let observer = Address::generate(&env);
        client.add_observer(&observer);

        let case_id = sample_bytes(&env, i as u8 + 1);
        client.create_case(&case_id, &owner, &None, &sample_bytes(&env, 20), &200);
        client.record_observation(
            &observer,
            &case_id,
            &sample_bytes(&env, 21),
            &100,
            &sample_bytes(&env, 22),
        );

        client.record_break(&observer, &case_id, &break_code);

        let case = client.get_case(&case_id);
        assert_eq!(case.status, CaseStatus::Break);
        assert_eq!(case.decision, Decision::Break(break_code));
    }
}

#[test]
fn test_match_without_observation_fails() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    let case_id = sample_bytes(&env, 1);
    client.create_case(&case_id, &owner, &None, &sample_bytes(&env, 2), &200);

    // Calling match directly on Open case must fail
    let res = client.try_record_match(&observer, &case_id);
    assert_eq!(res, Err(Ok(Error::InvalidState)));
}

#[test]
fn test_break_without_observation_fails() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    let case_id = sample_bytes(&env, 1);
    client.create_case(&case_id, &owner, &None, &sample_bytes(&env, 2), &200);

    // Calling break directly on Open case must fail
    let res = client.try_record_break(&observer, &case_id, &BreakCode::AmountMismatch);
    assert_eq!(res, Err(Ok(Error::InvalidState)));
}

// ---------------------------------------------------------------------------
// ATTESTATION TESTS
// ---------------------------------------------------------------------------

#[test]
fn test_submit_attestations_success() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let counterparty = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    let case_id = sample_bytes(&env, 1);
    client.create_case(
        &case_id,
        &owner,
        &Some(counterparty.clone()),
        &sample_bytes(&env, 2),
        &200,
    );
    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );

    // Owner attestation
    let owner_comm = sample_bytes(&env, 10);
    client.submit_attestation(&case_id, &AttestationRole::Owner, &owner_comm);
    let owner_att = client.get_attestation(&case_id, &owner).unwrap();
    assert_eq!(owner_att.role, AttestationRole::Owner);
    assert_eq!(owner_att.commitment, owner_comm);
    assert_eq!(owner_att.attested_at_ledger, 100);

    // Counterparty attestation
    let cp_comm = sample_bytes(&env, 11);
    client.submit_attestation(&case_id, &AttestationRole::Counterparty, &cp_comm);
    let cp_att = client.get_attestation(&case_id, &counterparty).unwrap();
    assert_eq!(cp_att.role, AttestationRole::Counterparty);
    assert_eq!(cp_att.commitment, cp_comm);

    // Observer attestation
    let obs_comm = sample_bytes(&env, 12);
    client.submit_attestation(&case_id, &AttestationRole::Observer, &obs_comm);
    let obs_att = client.get_attestation(&case_id, &observer).unwrap();
    assert_eq!(obs_att.role, AttestationRole::Observer);
    assert_eq!(obs_att.commitment, obs_comm);
}

#[test]
fn test_duplicate_attestation_fails() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let case_id = sample_bytes(&env, 1);
    client.create_case(&case_id, &owner, &None, &sample_bytes(&env, 2), &200);

    client.submit_attestation(&case_id, &AttestationRole::Owner, &sample_bytes(&env, 10));
    let res =
        client.try_submit_attestation(&case_id, &AttestationRole::Owner, &sample_bytes(&env, 11));
    assert_eq!(res, Err(Ok(Error::AttestationAlreadyExists)));
}

#[test]
fn test_counterparty_attestation_without_counterparty_fails() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let case_id = sample_bytes(&env, 1);
    client.create_case(&case_id, &owner, &None, &sample_bytes(&env, 2), &200);

    let res = client.try_submit_attestation(
        &case_id,
        &AttestationRole::Counterparty,
        &sample_bytes(&env, 10),
    );
    assert_eq!(res, Err(Ok(Error::CounterpartyRequired)));
}

#[test]
fn test_attestation_zero_commitment_fails() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let case_id = sample_bytes(&env, 1);
    client.create_case(&case_id, &owner, &None, &sample_bytes(&env, 2), &200);

    let res = client.try_submit_attestation(&case_id, &AttestationRole::Owner, &zero_bytes(&env));
    assert_eq!(res, Err(Ok(Error::InvalidCommitment)));
}

// ---------------------------------------------------------------------------
// DISPUTE & RESOLUTION TESTS
// ---------------------------------------------------------------------------

#[test]
fn test_dispute_and_resolution_success() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let counterparty = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    let case_id = sample_bytes(&env, 1);
    client.create_case(
        &case_id,
        &owner,
        &Some(counterparty.clone()),
        &sample_bytes(&env, 2),
        &200,
    );
    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );
    client.record_break(&observer, &case_id, &BreakCode::AmountMismatch);

    // Open dispute
    let dispute_comm = sample_bytes(&env, 30);
    client.open_dispute(&owner, &case_id, &dispute_comm);
    assert_eq!(client.get_case(&case_id).status, CaseStatus::Disputed);

    // Submit matching resolutions
    let res_comm = sample_bytes(&env, 31);
    client.submit_resolution(&owner, &case_id, &res_comm);
    assert_eq!(client.get_case(&case_id).status, CaseStatus::Disputed);
    assert_eq!(
        client.get_resolution(&case_id, &owner),
        Some(res_comm.clone())
    );

    // Counterparty submits matching resolution
    client.submit_resolution(&counterparty, &case_id, &res_comm);
    assert_eq!(client.get_case(&case_id).status, CaseStatus::Resolved);
    assert_eq!(
        client.get_resolution(&case_id, &counterparty),
        Some(res_comm)
    );
}

#[test]
fn test_counterparty_can_open_dispute() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let counterparty = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    let case_id = sample_bytes(&env, 1);
    client.create_case(
        &case_id,
        &owner,
        &Some(counterparty.clone()),
        &sample_bytes(&env, 2),
        &200,
    );
    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );
    client.record_break(&observer, &case_id, &BreakCode::AmountMismatch);

    // Counterparty opens dispute
    let dispute_comm = sample_bytes(&env, 30);
    client.open_dispute(&counterparty, &case_id, &dispute_comm);
    assert_eq!(client.get_case(&case_id).status, CaseStatus::Disputed);
}

#[test]
fn test_dispute_mismatched_resolution_remains_disputed() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let counterparty = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    let case_id = sample_bytes(&env, 1);
    client.create_case(
        &case_id,
        &owner,
        &Some(counterparty.clone()),
        &sample_bytes(&env, 2),
        &200,
    );
    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );
    client.record_break(&observer, &case_id, &BreakCode::AmountMismatch);

    client.open_dispute(&owner, &case_id, &sample_bytes(&env, 30));

    // Owner submits comm A, counterparty submits comm B
    client.submit_resolution(&owner, &case_id, &sample_bytes(&env, 31));
    client.submit_resolution(&counterparty, &case_id, &sample_bytes(&env, 32));

    // Must remain Disputed because commitments differ
    assert_eq!(client.get_case(&case_id).status, CaseStatus::Disputed);
}

#[test]
fn test_unauthorized_dispute_and_resolution_fails() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let counterparty = Address::generate(&env);
    let stranger = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    let case_id = sample_bytes(&env, 1);
    client.create_case(
        &case_id,
        &owner,
        &Some(counterparty.clone()),
        &sample_bytes(&env, 2),
        &200,
    );
    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );
    client.record_break(&observer, &case_id, &BreakCode::AmountMismatch);

    // Stranger cannot open dispute
    let res = client.try_open_dispute(&stranger, &case_id, &sample_bytes(&env, 30));
    assert_eq!(res, Err(Ok(Error::Unauthorized)));

    // Owner opens dispute
    client.open_dispute(&owner, &case_id, &sample_bytes(&env, 30));

    // Stranger cannot submit resolution
    let res = client.try_submit_resolution(&stranger, &case_id, &sample_bytes(&env, 31));
    assert_eq!(res, Err(Ok(Error::Unauthorized)));
}

#[test]
fn test_dispute_cannot_open_from_wrong_status() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let case_id = sample_bytes(&env, 1);
    client.create_case(&case_id, &owner, &None, &sample_bytes(&env, 2), &200);

    // Case is in Open state, cannot dispute
    let res = client.try_open_dispute(&owner, &case_id, &sample_bytes(&env, 30));
    assert_eq!(res, Err(Ok(Error::InvalidState)));
}

#[test]
fn test_duplicate_resolution_fails() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let counterparty = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    let case_id = sample_bytes(&env, 1);
    client.create_case(
        &case_id,
        &owner,
        &Some(counterparty.clone()),
        &sample_bytes(&env, 2),
        &200,
    );
    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );
    client.record_break(&observer, &case_id, &BreakCode::AmountMismatch);

    client.open_dispute(&owner, &case_id, &sample_bytes(&env, 30));

    // First submission succeeds
    client.submit_resolution(&owner, &case_id, &sample_bytes(&env, 31));

    // Second submission by same party fails
    let res = client.try_submit_resolution(&owner, &case_id, &sample_bytes(&env, 31));
    assert_eq!(res, Err(Ok(Error::ResolutionAlreadySubmitted)));
}

// ---------------------------------------------------------------------------
// FINALIZATION TESTS
// ---------------------------------------------------------------------------

#[test]
fn test_finalize_matched_case_success() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    let case_id = sample_bytes(&env, 1);
    client.create_case(&case_id, &owner, &None, &sample_bytes(&env, 2), &200);
    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );
    client.record_match(&observer, &case_id);

    // Submit required attestations (Owner + Observer)
    client.submit_attestation(&case_id, &AttestationRole::Owner, &sample_bytes(&env, 10));
    client.submit_attestation(
        &case_id,
        &AttestationRole::Observer,
        &sample_bytes(&env, 11),
    );

    // Advance ledger sequence
    env.ledger().set_sequence_number(120);

    client.finalize_case(&case_id);

    let case = client.get_case(&case_id);
    assert_eq!(case.status, CaseStatus::Finalized);
    assert_eq!(case.finalized_at_ledger, Some(120));
}

#[test]
fn test_finalize_matched_case_missing_attestation_fails() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    let case_id = sample_bytes(&env, 1);
    client.create_case(&case_id, &owner, &None, &sample_bytes(&env, 2), &200);
    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );
    client.record_match(&observer, &case_id);

    // Only owner attests, observer has not attested
    client.submit_attestation(&case_id, &AttestationRole::Owner, &sample_bytes(&env, 10));

    let res = client.try_finalize_case(&case_id);
    assert_eq!(res, Err(Ok(Error::MissingRequiredAttestation)));
}

#[test]
fn test_finalize_resolved_dispute_success() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let counterparty = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    let case_id = sample_bytes(&env, 1);
    client.create_case(
        &case_id,
        &owner,
        &Some(counterparty.clone()),
        &sample_bytes(&env, 2),
        &200,
    );
    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );
    client.record_break(&observer, &case_id, &BreakCode::AmountMismatch);

    client.open_dispute(&owner, &case_id, &sample_bytes(&env, 30));
    let res_comm = sample_bytes(&env, 31);
    client.submit_resolution(&owner, &case_id, &res_comm);
    client.submit_resolution(&counterparty, &case_id, &res_comm);
    assert_eq!(client.get_case(&case_id).status, CaseStatus::Resolved);

    // Submit all 3 required attestations (Owner, Counterparty, Observer)
    client.submit_attestation(&case_id, &AttestationRole::Owner, &sample_bytes(&env, 10));
    client.submit_attestation(
        &case_id,
        &AttestationRole::Counterparty,
        &sample_bytes(&env, 11),
    );
    client.submit_attestation(
        &case_id,
        &AttestationRole::Observer,
        &sample_bytes(&env, 12),
    );

    env.ledger().set_sequence_number(130);
    client.finalize_case(&case_id);

    let case = client.get_case(&case_id);
    assert_eq!(case.status, CaseStatus::Finalized);
    assert_eq!(case.finalized_at_ledger, Some(130));
}

#[test]
fn test_finalize_resolved_missing_counterparty_attestation_fails() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let counterparty = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    let case_id = sample_bytes(&env, 1);
    client.create_case(
        &case_id,
        &owner,
        &Some(counterparty.clone()),
        &sample_bytes(&env, 2),
        &200,
    );
    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );
    client.record_break(&observer, &case_id, &BreakCode::AmountMismatch);

    client.open_dispute(&owner, &case_id, &sample_bytes(&env, 30));
    let res_comm = sample_bytes(&env, 31);
    client.submit_resolution(&owner, &case_id, &res_comm);
    client.submit_resolution(&counterparty, &case_id, &res_comm);

    // Only owner and observer attest, counterparty missing
    client.submit_attestation(&case_id, &AttestationRole::Owner, &sample_bytes(&env, 10));
    client.submit_attestation(
        &case_id,
        &AttestationRole::Observer,
        &sample_bytes(&env, 12),
    );

    let res = client.try_finalize_case(&case_id);
    assert_eq!(res, Err(Ok(Error::MissingRequiredAttestation)));
}

// ---------------------------------------------------------------------------
// STATE MACHINE IMMUTABILITY & PROHIBITED TRANSITIONS
// ---------------------------------------------------------------------------

#[test]
fn test_finalized_case_cannot_mutate() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    let case_id = sample_bytes(&env, 1);
    client.create_case(&case_id, &owner, &None, &sample_bytes(&env, 2), &200);
    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );
    client.record_match(&observer, &case_id);
    client.submit_attestation(&case_id, &AttestationRole::Owner, &sample_bytes(&env, 10));
    client.submit_attestation(
        &case_id,
        &AttestationRole::Observer,
        &sample_bytes(&env, 11),
    );
    client.finalize_case(&case_id);

    // Try all state mutations on finalized case:
    assert_eq!(
        client.try_record_observation(
            &observer,
            &case_id,
            &sample_bytes(&env, 5),
            &100,
            &sample_bytes(&env, 6)
        ),
        Err(Ok(Error::InvalidState))
    );
    assert_eq!(
        client.try_record_match(&observer, &case_id),
        Err(Ok(Error::InvalidState))
    );
    assert_eq!(
        client.try_record_break(&observer, &case_id, &BreakCode::AmountMismatch),
        Err(Ok(Error::InvalidState))
    );
    assert_eq!(
        client.try_open_dispute(&owner, &case_id, &sample_bytes(&env, 30)),
        Err(Ok(Error::InvalidState))
    );
    assert_eq!(
        client.try_finalize_case(&case_id),
        Err(Ok(Error::InvalidState))
    );
}

#[test]
fn test_prohibited_transitions() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    let case_id = sample_bytes(&env, 1);
    client.create_case(&case_id, &owner, &None, &sample_bytes(&env, 2), &200);

    // Open cannot go directly to Matched
    assert_eq!(
        client.try_record_match(&observer, &case_id),
        Err(Ok(Error::InvalidState))
    );

    // Open cannot go directly to Break
    assert_eq!(
        client.try_record_break(&observer, &case_id, &BreakCode::AmountMismatch),
        Err(Ok(Error::InvalidState))
    );

    // Open cannot be finalized
    assert_eq!(
        client.try_finalize_case(&case_id),
        Err(Ok(Error::InvalidState))
    );

    // Record observation -> Observed
    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );

    // Observed cannot be disputed directly
    assert_eq!(
        client.try_open_dispute(&owner, &case_id, &sample_bytes(&env, 30)),
        Err(Ok(Error::InvalidState))
    );

    // Observed cannot be finalized directly
    assert_eq!(
        client.try_finalize_case(&case_id),
        Err(Ok(Error::InvalidState))
    );

    // Transition Observed -> Matched
    client.record_match(&observer, &case_id);

    // Matched cannot be disputed
    assert_eq!(
        client.try_open_dispute(&owner, &case_id, &sample_bytes(&env, 30)),
        Err(Ok(Error::InvalidState))
    );
}

#[test]
fn test_not_found_on_unknown_case() {
    let (env, _admin, client) = create_test_env();
    let unknown_case_id = sample_bytes(&env, 99);

    assert_eq!(
        client.try_get_case(&unknown_case_id),
        Err(Ok(Error::NotFound))
    );
}

// ---------------------------------------------------------------------------
// DEDICATED LIFECYCLE TRANSITION COVERAGE (LEGAL & ILLEGAL)
// ---------------------------------------------------------------------------

#[test]
fn test_legal_transition_open_to_observed() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    let case_id = sample_bytes(&env, 1);
    client.create_case(&case_id, &owner, &None, &sample_bytes(&env, 2), &200);
    assert_eq!(client.get_case(&case_id).status, CaseStatus::Open);

    // Open -> Observed
    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );
    let case = client.get_case(&case_id);
    assert_eq!(case.status, CaseStatus::Observed);
}

#[test]
fn test_legal_transition_observed_to_matched() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    let case_id = sample_bytes(&env, 1);
    client.create_case(&case_id, &owner, &None, &sample_bytes(&env, 2), &200);
    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );
    assert_eq!(client.get_case(&case_id).status, CaseStatus::Observed);

    // Observed -> Matched
    client.record_match(&observer, &case_id);
    let case = client.get_case(&case_id);
    assert_eq!(case.status, CaseStatus::Matched);
    assert_eq!(case.decision, Decision::Matched);
}

#[test]
fn test_legal_transition_observed_to_break() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    let case_id = sample_bytes(&env, 1);
    client.create_case(&case_id, &owner, &None, &sample_bytes(&env, 2), &200);
    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );
    assert_eq!(client.get_case(&case_id).status, CaseStatus::Observed);

    // Observed -> Break
    client.record_break(&observer, &case_id, &BreakCode::DestinationMismatch);
    let case = client.get_case(&case_id);
    assert_eq!(case.status, CaseStatus::Break);
    assert_eq!(
        case.decision,
        Decision::Break(BreakCode::DestinationMismatch)
    );
}

#[test]
fn test_legal_transition_break_to_disputed() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let counterparty = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    let case_id = sample_bytes(&env, 1);
    client.create_case(
        &case_id,
        &owner,
        &Some(counterparty.clone()),
        &sample_bytes(&env, 2),
        &200,
    );
    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );
    client.record_break(&observer, &case_id, &BreakCode::LateSettlement);
    assert_eq!(client.get_case(&case_id).status, CaseStatus::Break);

    // Break -> Disputed
    client.open_dispute(&owner, &case_id, &sample_bytes(&env, 30));
    assert_eq!(client.get_case(&case_id).status, CaseStatus::Disputed);
}

#[test]
fn test_legal_transition_disputed_to_resolved() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let counterparty = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    let case_id = sample_bytes(&env, 1);
    client.create_case(
        &case_id,
        &owner,
        &Some(counterparty.clone()),
        &sample_bytes(&env, 2),
        &200,
    );
    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );
    client.record_break(&observer, &case_id, &BreakCode::AmountMismatch);
    client.open_dispute(&owner, &case_id, &sample_bytes(&env, 30));
    assert_eq!(client.get_case(&case_id).status, CaseStatus::Disputed);

    // Disputed -> Resolved (after both parties submit matching commitments)
    let res_comm = sample_bytes(&env, 31);
    client.submit_resolution(&owner, &case_id, &res_comm);
    client.submit_resolution(&counterparty, &case_id, &res_comm);
    assert_eq!(client.get_case(&case_id).status, CaseStatus::Resolved);
}

#[test]
fn test_legal_transition_matched_to_finalized() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    let case_id = sample_bytes(&env, 1);
    client.create_case(&case_id, &owner, &None, &sample_bytes(&env, 2), &200);
    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );
    client.record_match(&observer, &case_id);
    assert_eq!(client.get_case(&case_id).status, CaseStatus::Matched);

    // Required attestations
    client.submit_attestation(&case_id, &AttestationRole::Owner, &sample_bytes(&env, 10));
    client.submit_attestation(
        &case_id,
        &AttestationRole::Observer,
        &sample_bytes(&env, 11),
    );

    // Matched -> Finalized
    env.ledger().set_sequence_number(150);
    client.finalize_case(&case_id);
    let case = client.get_case(&case_id);
    assert_eq!(case.status, CaseStatus::Finalized);
    assert_eq!(case.finalized_at_ledger, Some(150));
}

#[test]
fn test_legal_transition_resolved_to_finalized() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let counterparty = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    let case_id = sample_bytes(&env, 1);
    client.create_case(
        &case_id,
        &owner,
        &Some(counterparty.clone()),
        &sample_bytes(&env, 2),
        &200,
    );
    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );
    client.record_break(&observer, &case_id, &BreakCode::FailedTransaction);
    client.open_dispute(&counterparty, &case_id, &sample_bytes(&env, 30));

    let res_comm = sample_bytes(&env, 31);
    client.submit_resolution(&owner, &case_id, &res_comm);
    client.submit_resolution(&counterparty, &case_id, &res_comm);
    assert_eq!(client.get_case(&case_id).status, CaseStatus::Resolved);

    // Required attestations
    client.submit_attestation(&case_id, &AttestationRole::Owner, &sample_bytes(&env, 10));
    client.submit_attestation(
        &case_id,
        &AttestationRole::Counterparty,
        &sample_bytes(&env, 11),
    );
    client.submit_attestation(
        &case_id,
        &AttestationRole::Observer,
        &sample_bytes(&env, 12),
    );

    // Resolved -> Finalized
    env.ledger().set_sequence_number(160);
    client.finalize_case(&case_id);
    let case = client.get_case(&case_id);
    assert_eq!(case.status, CaseStatus::Finalized);
    assert_eq!(case.finalized_at_ledger, Some(160));
}

#[test]
fn test_validate_state_transition_matrix() {
    use crate::auth::validate_state_transition;

    let all_statuses = [
        CaseStatus::Open,
        CaseStatus::Observed,
        CaseStatus::Matched,
        CaseStatus::Break,
        CaseStatus::Disputed,
        CaseStatus::Resolved,
        CaseStatus::Finalized,
    ];

    for &from in &all_statuses {
        for &to in &all_statuses {
            let is_legal = matches!(
                (from, to),
                (CaseStatus::Open, CaseStatus::Observed)
                    | (CaseStatus::Observed, CaseStatus::Matched)
                    | (CaseStatus::Observed, CaseStatus::Break)
                    | (CaseStatus::Break, CaseStatus::Disputed)
                    | (CaseStatus::Disputed, CaseStatus::Resolved)
                    | (CaseStatus::Matched, CaseStatus::Finalized)
                    | (CaseStatus::Resolved, CaseStatus::Finalized)
            );

            let res = validate_state_transition(from, to);
            if is_legal {
                assert_eq!(
                    res,
                    Ok(()),
                    "Expected legal transition from {:?} to {:?}",
                    from,
                    to
                );
            } else {
                assert_eq!(
                    res,
                    Err(Error::InvalidState),
                    "Expected illegal transition from {:?} to {:?}",
                    from,
                    to
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// AUTHORIZATION INVARIANTS TESTS
// ---------------------------------------------------------------------------

#[test]
fn test_auth_observer_registration_and_revocation() {
    let (env, _admin, client) = create_test_env();
    let observer = Address::generate(&env);

    // Admin can register observer
    client.add_observer(&observer);
    assert!(client.is_observer(&observer));

    // Admin can remove observer
    client.remove_observer(&observer);
    assert!(!client.is_observer(&observer));

    // Unregistered observer removal fails
    let res = client.try_remove_observer(&observer);
    assert_eq!(res, Err(Ok(Error::ObserverNotRegistered)));
}

#[test]
fn test_auth_observation_recording_requires_registered_observer() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let unauthorized_observer = Address::generate(&env);

    let case_id = sample_bytes(&env, 1);
    client.create_case(&case_id, &owner, &None, &sample_bytes(&env, 2), &200);

    // Unregistered address cannot record observation
    let res = client.try_record_observation(
        &unauthorized_observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );
    assert_eq!(res, Err(Ok(Error::ObserverNotRegistered)));
}

#[test]
fn test_auth_match_recording_requires_registered_observer() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let observer = Address::generate(&env);
    let unauthorized = Address::generate(&env);
    client.add_observer(&observer);

    let case_id = sample_bytes(&env, 1);
    client.create_case(&case_id, &owner, &None, &sample_bytes(&env, 2), &200);
    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );

    // Unregistered address cannot record match
    let res = client.try_record_match(&unauthorized, &case_id);
    assert_eq!(res, Err(Ok(Error::ObserverNotRegistered)));
}

#[test]
fn test_auth_break_recording_requires_registered_observer() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let observer = Address::generate(&env);
    let unauthorized = Address::generate(&env);
    client.add_observer(&observer);

    let case_id = sample_bytes(&env, 1);
    client.create_case(&case_id, &owner, &None, &sample_bytes(&env, 2), &200);
    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );

    // Unregistered address cannot record break
    let res = client.try_record_break(&unauthorized, &case_id, &BreakCode::AssetMismatch);
    assert_eq!(res, Err(Ok(Error::ObserverNotRegistered)));
}

#[test]
fn test_auth_attestation_submission_invariants() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let counterparty = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    let case_id = sample_bytes(&env, 1);
    client.create_case(
        &case_id,
        &owner,
        &Some(counterparty.clone()),
        &sample_bytes(&env, 2),
        &200,
    );
    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );

    // Attesting with Observer role before observer is set fails on a case with no observer:
    let empty_case_id = sample_bytes(&env, 9);
    client.create_case(&empty_case_id, &owner, &None, &sample_bytes(&env, 2), &200);
    let res = client.try_submit_attestation(
        &empty_case_id,
        &AttestationRole::Observer,
        &sample_bytes(&env, 10),
    );
    assert_eq!(res, Err(Ok(Error::ObserverNotRegistered)));

    // Counterparty role attestation on case with no counterparty fails
    let res = client.try_submit_attestation(
        &empty_case_id,
        &AttestationRole::Counterparty,
        &sample_bytes(&env, 10),
    );
    assert_eq!(res, Err(Ok(Error::CounterpartyRequired)));
}

#[test]
fn test_auth_dispute_opening_and_resolution_unauthorized_rejected() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let counterparty = Address::generate(&env);
    let observer = Address::generate(&env);
    let unauthorized = Address::generate(&env);
    client.add_observer(&observer);

    let case_id = sample_bytes(&env, 1);
    client.create_case(
        &case_id,
        &owner,
        &Some(counterparty.clone()),
        &sample_bytes(&env, 2),
        &200,
    );
    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );
    client.record_break(&observer, &case_id, &BreakCode::ReferenceMismatch);

    // Unauthorized address cannot open dispute
    let res = client.try_open_dispute(&unauthorized, &case_id, &sample_bytes(&env, 30));
    assert_eq!(res, Err(Ok(Error::Unauthorized)));

    // Owner opens dispute
    client.open_dispute(&owner, &case_id, &sample_bytes(&env, 30));

    // Unauthorized address cannot submit resolution
    let res = client.try_submit_resolution(&unauthorized, &case_id, &sample_bytes(&env, 31));
    assert_eq!(res, Err(Ok(Error::Unauthorized)));

    // Observer cannot submit resolution
    let res = client.try_submit_resolution(&observer, &case_id, &sample_bytes(&env, 31));
    assert_eq!(res, Err(Ok(Error::Unauthorized)));
}

#[test]
fn test_auth_finalization_requires_active_registered_observer() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    let case_id = sample_bytes(&env, 1);
    client.create_case(&case_id, &owner, &None, &sample_bytes(&env, 2), &200);
    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );
    client.record_match(&observer, &case_id);

    client.submit_attestation(&case_id, &AttestationRole::Owner, &sample_bytes(&env, 10));
    client.submit_attestation(
        &case_id,
        &AttestationRole::Observer,
        &sample_bytes(&env, 11),
    );

    // Revoke observer before finalization
    client.remove_observer(&observer);

    // Finalization must fail because observer is no longer active
    let res = client.try_finalize_case(&case_id);
    assert_eq!(res, Err(Ok(Error::MissingRequiredAttestation)));
}

// ---------------------------------------------------------------------------
// COMMITMENT HARDENING & MALFORMED EVIDENCE REJECTION TESTS
// ---------------------------------------------------------------------------

#[test]
fn test_validation_zero_case_id_rejected_on_all_endpoints() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);
    let zero_case = zero_bytes(&env);

    // create_case
    assert_eq!(
        client.try_create_case(&zero_case, &owner, &None, &sample_bytes(&env, 1), &200),
        Err(Ok(Error::InvalidCommitment))
    );

    // record_observation
    assert_eq!(
        client.try_record_observation(
            &observer,
            &zero_case,
            &sample_bytes(&env, 1),
            &100,
            &sample_bytes(&env, 2)
        ),
        Err(Ok(Error::InvalidCommitment))
    );

    // record_match
    assert_eq!(
        client.try_record_match(&observer, &zero_case),
        Err(Ok(Error::InvalidCommitment))
    );

    // record_break
    assert_eq!(
        client.try_record_break(&observer, &zero_case, &BreakCode::AmountMismatch),
        Err(Ok(Error::InvalidCommitment))
    );

    // submit_attestation
    assert_eq!(
        client.try_submit_attestation(&zero_case, &AttestationRole::Owner, &sample_bytes(&env, 1)),
        Err(Ok(Error::InvalidCommitment))
    );

    // open_dispute
    assert_eq!(
        client.try_open_dispute(&owner, &zero_case, &sample_bytes(&env, 1)),
        Err(Ok(Error::InvalidCommitment))
    );

    // submit_resolution
    assert_eq!(
        client.try_submit_resolution(&owner, &zero_case, &sample_bytes(&env, 1)),
        Err(Ok(Error::InvalidCommitment))
    );

    // finalize_case
    assert_eq!(
        client.try_finalize_case(&zero_case),
        Err(Ok(Error::InvalidCommitment))
    );
}

#[test]
fn test_validation_zero_tx_hash_rejected() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    let case_id = sample_bytes(&env, 1);
    client.create_case(&case_id, &owner, &None, &sample_bytes(&env, 2), &200);

    let res = client.try_record_observation(
        &observer,
        &case_id,
        &zero_bytes(&env),
        &100,
        &sample_bytes(&env, 3),
    );
    assert_eq!(res, Err(Ok(Error::InvalidCommitment)));
}
