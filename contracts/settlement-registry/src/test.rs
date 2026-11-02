#![cfg(test)]

use crate::errors::Error;
use crate::storage::PROTOCOL_VERSION;
use crate::types::{
    AttestationRole, BreakCode, CaseStatus, Decision, Observation, ObservationRecord,
};
use crate::{SettlementRegistry, SettlementRegistryClient};
use soroban_sdk::testutils::{Address as _, Events as _, Ledger};
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
    assert_eq!(res, Err(Ok(Error::ObserverQuorumNotMet)));
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
        client.try_set_case_quorum(&case_id, &2),
        Err(Ok(Error::InvalidState))
    );
    assert_eq!(
        client.try_submit_attestation(&case_id, &AttestationRole::Owner, &sample_bytes(&env, 20)),
        Err(Ok(Error::InvalidState))
    );
    assert_eq!(
        client.try_submit_observer_attestation(&case_id, &observer, &sample_bytes(&env, 21)),
        Err(Ok(Error::InvalidState))
    );
    assert_eq!(
        client.try_open_dispute(&owner, &case_id, &sample_bytes(&env, 30)),
        Err(Ok(Error::InvalidState))
    );
    assert_eq!(
        client.try_open_dispute_with_ttl(&owner, &case_id, &sample_bytes(&env, 30), &200),
        Err(Ok(Error::InvalidState))
    );
    assert_eq!(
        client.try_submit_resolution(&owner, &case_id, &sample_bytes(&env, 31)),
        Err(Ok(Error::InvalidState))
    );
    assert_eq!(
        client.try_expire_dispute(&case_id),
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
                    | (CaseStatus::Disputed, CaseStatus::Break)
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

    // Revoke observer after attestation submission
    client.remove_observer(&observer);

    // Finalization succeeds because observer was authorized at attestation submission time
    client.finalize_case(&case_id);
    assert_eq!(client.get_case(&case_id).status, CaseStatus::Finalized);
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

// ---------------------------------------------------------------------------
// ADVERSARIAL SETTLEMENT TEST SUITE
// ---------------------------------------------------------------------------

#[test]
fn test_adversarial_duplicate_observation() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let observer1 = Address::generate(&env);
    let observer2 = Address::generate(&env);
    client.add_observer(&observer1);
    client.add_observer(&observer2);

    let case_id = sample_bytes(&env, 1);
    client.create_case(&case_id, &owner, &None, &sample_bytes(&env, 2), &200);

    // Initial observation succeeds
    client.record_observation(
        &observer1,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );

    // Adversarial: observer1 attempts second observation
    let res = client.try_record_observation(
        &observer1,
        &case_id,
        &sample_bytes(&env, 5),
        &100,
        &sample_bytes(&env, 6),
    );
    assert_eq!(res, Err(Ok(Error::InvalidState)));

    // Adversarial: observer2 attempts second observation
    let res = client.try_record_observation(
        &observer2,
        &case_id,
        &sample_bytes(&env, 7),
        &100,
        &sample_bytes(&env, 8),
    );
    assert_eq!(res, Err(Ok(Error::InvalidState)));
}

#[test]
fn test_adversarial_duplicate_reconciliation() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    // Case 1: Match then duplicate match or break
    let case_id1 = sample_bytes(&env, 1);
    client.create_case(&case_id1, &owner, &None, &sample_bytes(&env, 2), &200);
    client.record_observation(
        &observer,
        &case_id1,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );
    client.record_match(&observer, &case_id1);

    // Duplicate match fails
    assert_eq!(
        client.try_record_match(&observer, &case_id1),
        Err(Ok(Error::InvalidState))
    );
    // Break after match fails
    assert_eq!(
        client.try_record_break(&observer, &case_id1, &BreakCode::AmountMismatch),
        Err(Ok(Error::InvalidState))
    );

    // Case 2: Break then duplicate break or match
    let case_id2 = sample_bytes(&env, 5);
    client.create_case(&case_id2, &owner, &None, &sample_bytes(&env, 6), &200);
    client.record_observation(
        &observer,
        &case_id2,
        &sample_bytes(&env, 7),
        &100,
        &sample_bytes(&env, 8),
    );
    client.record_break(&observer, &case_id2, &BreakCode::LateSettlement);

    // Duplicate break fails
    assert_eq!(
        client.try_record_break(&observer, &case_id2, &BreakCode::LateSettlement),
        Err(Ok(Error::InvalidState))
    );
    // Match after break fails
    assert_eq!(
        client.try_record_match(&observer, &case_id2),
        Err(Ok(Error::InvalidState))
    );
}

#[test]
fn test_adversarial_wrong_observer() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let legitimate_observer = Address::generate(&env);
    let rogue_observer = Address::generate(&env);
    client.add_observer(&legitimate_observer);

    let case_id = sample_bytes(&env, 1);
    client.create_case(&case_id, &owner, &None, &sample_bytes(&env, 2), &200);

    // Rogue observer tries to record observation
    assert_eq!(
        client.try_record_observation(
            &rogue_observer,
            &case_id,
            &sample_bytes(&env, 3),
            &100,
            &sample_bytes(&env, 4),
        ),
        Err(Ok(Error::ObserverNotRegistered))
    );

    // Legitimate observer records observation
    client.record_observation(
        &legitimate_observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );

    // Rogue observer tries to record match
    assert_eq!(
        client.try_record_match(&rogue_observer, &case_id),
        Err(Ok(Error::ObserverNotRegistered))
    );

    // Rogue observer tries to record break
    assert_eq!(
        client.try_record_break(&rogue_observer, &case_id, &BreakCode::AmountMismatch),
        Err(Ok(Error::ObserverNotRegistered))
    );
}

#[test]
fn test_adversarial_wrong_commitment() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let counterparty = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);
    let zero_comm = zero_bytes(&env);

    let case_id = sample_bytes(&env, 1);

    // Zero terms commitment
    assert_eq!(
        client.try_create_case(
            &case_id,
            &owner,
            &Some(counterparty.clone()),
            &zero_comm,
            &200
        ),
        Err(Ok(Error::InvalidCommitment))
    );

    // Create case properly
    client.create_case(
        &case_id,
        &owner,
        &Some(counterparty.clone()),
        &sample_bytes(&env, 2),
        &200,
    );

    // Zero observation commitment
    assert_eq!(
        client.try_record_observation(
            &observer,
            &case_id,
            &sample_bytes(&env, 3),
            &100,
            &zero_comm
        ),
        Err(Ok(Error::InvalidCommitment))
    );

    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );
    client.record_break(&observer, &case_id, &BreakCode::AmountMismatch);

    // Zero dispute commitment
    assert_eq!(
        client.try_open_dispute(&owner, &case_id, &zero_comm),
        Err(Ok(Error::InvalidCommitment))
    );

    client.open_dispute(&owner, &case_id, &sample_bytes(&env, 30));

    // Zero resolution commitment
    assert_eq!(
        client.try_submit_resolution(&owner, &case_id, &zero_comm),
        Err(Ok(Error::InvalidCommitment))
    );
}

#[test]
fn test_adversarial_wrong_case_id() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    let legitimate_case_id = sample_bytes(&env, 1);
    let bogus_case_id = sample_bytes(&env, 99);

    client.create_case(
        &legitimate_case_id,
        &owner,
        &None,
        &sample_bytes(&env, 2),
        &200,
    );

    // Access non-existent case on each endpoint
    assert_eq!(
        client.try_record_observation(
            &observer,
            &bogus_case_id,
            &sample_bytes(&env, 3),
            &100,
            &sample_bytes(&env, 4)
        ),
        Err(Ok(Error::NotFound))
    );
    assert_eq!(
        client.try_record_match(&observer, &bogus_case_id),
        Err(Ok(Error::NotFound))
    );
    assert_eq!(
        client.try_record_break(&observer, &bogus_case_id, &BreakCode::LateSettlement),
        Err(Ok(Error::NotFound))
    );
    assert_eq!(
        client.try_submit_attestation(
            &bogus_case_id,
            &AttestationRole::Owner,
            &sample_bytes(&env, 10)
        ),
        Err(Ok(Error::NotFound))
    );
    assert_eq!(
        client.try_open_dispute(&owner, &bogus_case_id, &sample_bytes(&env, 30)),
        Err(Ok(Error::NotFound))
    );
    assert_eq!(
        client.try_submit_resolution(&owner, &bogus_case_id, &sample_bytes(&env, 31)),
        Err(Ok(Error::NotFound))
    );
    assert_eq!(
        client.try_finalize_case(&bogus_case_id),
        Err(Ok(Error::NotFound))
    );
    assert_eq!(
        client.try_get_case(&bogus_case_id),
        Err(Ok(Error::NotFound))
    );
}

#[test]
fn test_adversarial_unauthorized_attestation() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    let case_id = sample_bytes(&env, 1);
    // Case created without counterparty
    client.create_case(&case_id, &owner, &None, &sample_bytes(&env, 2), &200);

    // Counterparty role attestation on single-party case
    assert_eq!(
        client.try_submit_attestation(
            &case_id,
            &AttestationRole::Counterparty,
            &sample_bytes(&env, 10)
        ),
        Err(Ok(Error::CounterpartyRequired))
    );

    // Observer role attestation before observer is recorded
    assert_eq!(
        client.try_submit_attestation(
            &case_id,
            &AttestationRole::Observer,
            &sample_bytes(&env, 11)
        ),
        Err(Ok(Error::ObserverNotRegistered))
    );
}

#[test]
fn test_adversarial_premature_finalization() {
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

    // Premature: finalize Open case
    assert_eq!(
        client.try_finalize_case(&case_id),
        Err(Ok(Error::InvalidState))
    );

    // Premature: finalize Observed case
    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );
    assert_eq!(
        client.try_finalize_case(&case_id),
        Err(Ok(Error::InvalidState))
    );

    // Premature: finalize Break case
    client.record_break(&observer, &case_id, &BreakCode::AmountMismatch);
    assert_eq!(
        client.try_finalize_case(&case_id),
        Err(Ok(Error::InvalidState))
    );

    // Premature: finalize Disputed case
    client.open_dispute(&owner, &case_id, &sample_bytes(&env, 30));
    assert_eq!(
        client.try_finalize_case(&case_id),
        Err(Ok(Error::InvalidState))
    );

    // Premature: finalize Resolved case without attestations
    let res_comm = sample_bytes(&env, 31);
    client.submit_resolution(&owner, &case_id, &res_comm);
    client.submit_resolution(&counterparty, &case_id, &res_comm);
    assert_eq!(
        client.try_finalize_case(&case_id),
        Err(Ok(Error::MissingRequiredAttestation))
    );
}

#[test]
fn test_adversarial_duplicate_finalization() {
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

    // First finalization succeeds
    client.finalize_case(&case_id);
    assert_eq!(client.get_case(&case_id).status, CaseStatus::Finalized);

    // Adversarial: second finalization fails
    let res = client.try_finalize_case(&case_id);
    assert_eq!(res, Err(Ok(Error::InvalidState)));
}

#[test]
fn test_adversarial_invalid_dispute_resolution() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let counterparty = Address::generate(&env);
    let outsider = Address::generate(&env);
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

    // Resolution on Open case fails
    assert_eq!(
        client.try_submit_resolution(&owner, &case_id, &sample_bytes(&env, 31)),
        Err(Ok(Error::InvalidState))
    );

    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );
    client.record_break(&observer, &case_id, &BreakCode::LateSettlement);

    // Resolution before opening dispute fails
    assert_eq!(
        client.try_submit_resolution(&owner, &case_id, &sample_bytes(&env, 31)),
        Err(Ok(Error::InvalidState))
    );

    client.open_dispute(&owner, &case_id, &sample_bytes(&env, 30));

    // Resolution by outsider fails
    assert_eq!(
        client.try_submit_resolution(&outsider, &case_id, &sample_bytes(&env, 31)),
        Err(Ok(Error::Unauthorized))
    );

    // Owner submits resolution A
    client.submit_resolution(&owner, &case_id, &sample_bytes(&env, 31));

    // Duplicate resolution by owner fails
    assert_eq!(
        client.try_submit_resolution(&owner, &case_id, &sample_bytes(&env, 31)),
        Err(Ok(Error::ResolutionAlreadySubmitted))
    );

    // Counterparty submits mismatching resolution B
    client.submit_resolution(&counterparty, &case_id, &sample_bytes(&env, 32));

    // Status must remain Disputed due to mismatch
    assert_eq!(client.get_case(&case_id).status, CaseStatus::Disputed);
}

// ---------------------------------------------------------------------------
// EVENT COMPLETENESS TEST SUITE
// ---------------------------------------------------------------------------

#[test]
fn test_event_completeness_case_creation() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let counterparty = Address::generate(&env);
    let case_id = sample_bytes(&env, 1);
    let terms = sample_bytes(&env, 2);

    client.create_case(&case_id, &owner, &Some(counterparty.clone()), &terms, &200);
    assert_eq!(env.events().all().events().len(), 1);
}

#[test]
fn test_event_completeness_observation() {
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
    assert_eq!(env.events().all().events().len(), 1);
}

#[test]
fn test_event_completeness_match() {
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
    assert_eq!(env.events().all().events().len(), 1);
}

#[test]
fn test_event_completeness_break() {
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

    client.record_break(&observer, &case_id, &BreakCode::DestinationMismatch);
    assert_eq!(env.events().all().events().len(), 1);
}

#[test]
fn test_event_completeness_attestation() {
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

    client.submit_attestation(&case_id, &AttestationRole::Owner, &sample_bytes(&env, 10));
    assert_eq!(env.events().all().events().len(), 1);

    client.submit_attestation(
        &case_id,
        &AttestationRole::Counterparty,
        &sample_bytes(&env, 11),
    );
    assert_eq!(env.events().all().events().len(), 1);

    client.submit_attestation(
        &case_id,
        &AttestationRole::Observer,
        &sample_bytes(&env, 12),
    );
    assert_eq!(env.events().all().events().len(), 1);
}

#[test]
fn test_event_completeness_dispute_and_resolution() {
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

    // Dispute event
    client.open_dispute(&owner, &case_id, &sample_bytes(&env, 30));
    assert_eq!(env.events().all().events().len(), 1);

    // Resolution submission 1 (emits ResolutionSubmitted)
    let res_comm = sample_bytes(&env, 31);
    client.submit_resolution(&owner, &case_id, &res_comm);
    assert_eq!(env.events().all().events().len(), 1);

    // Resolution submission 2 (emits ResolutionSubmitted AND DisputeResolved)
    client.submit_resolution(&counterparty, &case_id, &res_comm);
    assert_eq!(env.events().all().events().len(), 2);
}

#[test]
fn test_event_completeness_finalization() {
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
    assert_eq!(env.events().all().events().len(), 1);
}

#[test]
fn test_event_completeness_observer_lifecycle() {
    let (env, _admin, client) = create_test_env();
    let observer = Address::generate(&env);

    client.add_observer(&observer);
    assert_eq!(env.events().all().events().len(), 1);

    client.remove_observer(&observer);
    assert_eq!(env.events().all().events().len(), 1);
}

// ---------------------------------------------------------------------------
// EVENT AND STATE INVARIANTS TESTS
// ---------------------------------------------------------------------------

#[test]
fn test_invariant_case_created_event_matches_stored_state() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let counterparty = Address::generate(&env);
    let case_id = sample_bytes(&env, 1);
    let terms_commitment = sample_bytes(&env, 2);
    let expires_at = 250;

    client.create_case(
        &case_id,
        &owner,
        &Some(counterparty.clone()),
        &terms_commitment,
        &expires_at,
    );

    let stored = client.get_case(&case_id);
    assert_eq!(stored.owner, owner);
    assert_eq!(stored.counterparty, Some(counterparty));
    assert_eq!(stored.terms_commitment, terms_commitment);
    assert_eq!(stored.expires_at_ledger, expires_at);
    assert_eq!(stored.status, CaseStatus::Open);
}

#[test]
fn test_invariant_observation_event_matches_stored_state() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    let case_id = sample_bytes(&env, 1);
    let terms = sample_bytes(&env, 2);
    let tx_hash = sample_bytes(&env, 3);
    let obs_comm = sample_bytes(&env, 4);
    let obs_ledger = 100;

    client.create_case(&case_id, &owner, &None, &terms, &200);
    client.record_observation(&observer, &case_id, &tx_hash, &obs_ledger, &obs_comm);

    let stored = client.get_case(&case_id);
    assert_eq!(stored.status, CaseStatus::Observed);
    assert_eq!(
        stored.observation,
        Observation::Observed(ObservationRecord {
            tx_hash: tx_hash.clone(),
            observation_commitment: obs_comm.clone(),
            observed_ledger: obs_ledger,
        })
    );
}

#[test]
fn test_invariant_match_and_break_event_matches_stored_state() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    // Matched invariant
    let case_id1 = sample_bytes(&env, 1);
    client.create_case(&case_id1, &owner, &None, &sample_bytes(&env, 2), &200);
    client.record_observation(
        &observer,
        &case_id1,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );
    client.record_match(&observer, &case_id1);

    let stored1 = client.get_case(&case_id1);
    assert_eq!(stored1.status, CaseStatus::Matched);
    assert_eq!(stored1.decision, Decision::Matched);

    // Break invariant
    let case_id2 = sample_bytes(&env, 5);
    client.create_case(&case_id2, &owner, &None, &sample_bytes(&env, 6), &200);
    client.record_observation(
        &observer,
        &case_id2,
        &sample_bytes(&env, 7),
        &100,
        &sample_bytes(&env, 8),
    );
    client.record_break(&observer, &case_id2, &BreakCode::DestinationMismatch);

    let stored2 = client.get_case(&case_id2);
    assert_eq!(stored2.status, CaseStatus::Break);
    assert_eq!(
        stored2.decision,
        Decision::Break(BreakCode::DestinationMismatch)
    );
}

#[test]
fn test_invariant_attestation_event_matches_stored_state() {
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

    let owner_comm = sample_bytes(&env, 10);
    client.submit_attestation(&case_id, &AttestationRole::Owner, &owner_comm);

    let stored_att = client.get_attestation(&case_id, &owner).unwrap();
    assert_eq!(stored_att.role, AttestationRole::Owner);
    assert_eq!(stored_att.commitment, owner_comm);
    assert_eq!(stored_att.attested_at_ledger, 100);
}

#[test]
fn test_invariant_dispute_and_resolution_event_matches_stored_state() {
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

    let dispute_comm = sample_bytes(&env, 30);
    client.open_dispute(&owner, &case_id, &dispute_comm);
    assert_eq!(client.get_case(&case_id).status, CaseStatus::Disputed);

    let res_comm = sample_bytes(&env, 31);
    client.submit_resolution(&owner, &case_id, &res_comm);
    client.submit_resolution(&counterparty, &case_id, &res_comm);

    assert_eq!(client.get_case(&case_id).status, CaseStatus::Resolved);
    assert_eq!(
        client.get_resolution(&case_id, &owner),
        Some(res_comm.clone())
    );
    assert_eq!(
        client.get_resolution(&case_id, &counterparty),
        Some(res_comm)
    );
}

#[test]
fn test_invariant_finalization_event_matches_stored_state() {
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

    env.ledger().set_sequence_number(175);
    client.finalize_case(&case_id);

    let stored = client.get_case(&case_id);
    assert_eq!(stored.status, CaseStatus::Finalized);
    assert_eq!(stored.finalized_at_ledger, Some(175));
}

// ---------------------------------------------------------------------------
// BOUNDARY AND DUPLICATE-STATE FUZZ COVERAGE TESTS
// ---------------------------------------------------------------------------

#[test]
fn test_boundary_empty_commitments_across_all_calls() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    let zero = zero_bytes(&env);
    let normal = sample_bytes(&env, 1);

    // Empty case_id in create_case
    assert_eq!(
        client.try_create_case(&zero, &owner, &None, &normal, &200),
        Err(Ok(Error::InvalidCommitment))
    );

    // Empty terms_commitment in create_case
    assert_eq!(
        client.try_create_case(&normal, &owner, &None, &zero, &200),
        Err(Ok(Error::InvalidCommitment))
    );

    // Setup valid case
    client.create_case(&normal, &owner, &None, &sample_bytes(&env, 2), &200);

    // Empty tx_hash in observation
    assert_eq!(
        client.try_record_observation(&observer, &normal, &zero, &100, &sample_bytes(&env, 4)),
        Err(Ok(Error::InvalidCommitment))
    );

    // Empty observation_commitment in observation
    assert_eq!(
        client.try_record_observation(&observer, &normal, &sample_bytes(&env, 3), &100, &zero),
        Err(Ok(Error::InvalidCommitment))
    );

    // Empty attestation commitment
    assert_eq!(
        client.try_submit_attestation(&normal, &AttestationRole::Owner, &zero),
        Err(Ok(Error::InvalidCommitment))
    );

    // Empty dispute commitment
    assert_eq!(
        client.try_open_dispute(&owner, &normal, &zero),
        Err(Ok(Error::InvalidCommitment))
    );

    // Empty resolution commitment
    assert_eq!(
        client.try_submit_resolution(&owner, &normal, &zero),
        Err(Ok(Error::InvalidCommitment))
    );
}

#[test]
fn test_boundary_maximum_size_supported_values() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let counterparty = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    // Maximum byte value (all 0xFF)
    let max_bytes = sample_bytes(&env, 0xFF);
    let max_expiry = u32::MAX;

    // Case creation with max byte commitments and max expiry
    client.create_case(
        &max_bytes,
        &owner,
        &Some(counterparty.clone()),
        &max_bytes,
        &max_expiry,
    );

    let case = client.get_case(&max_bytes);
    assert_eq!(case.terms_commitment, max_bytes);
    assert_eq!(case.expires_at_ledger, max_expiry);

    // Observation with max byte values
    client.record_observation(&observer, &max_bytes, &max_bytes, &100, &max_bytes);

    // Match with max byte values
    client.record_match(&observer, &max_bytes);

    // Attestation with max byte values
    client.submit_attestation(&max_bytes, &AttestationRole::Owner, &max_bytes);
    client.submit_attestation(&max_bytes, &AttestationRole::Observer, &max_bytes);

    // Finalize case
    client.finalize_case(&max_bytes);
    assert_eq!(client.get_case(&max_bytes).status, CaseStatus::Finalized);
}

#[test]
fn test_boundary_repeated_identical_calls() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    // Repeated add_observer
    assert_eq!(
        client.try_add_observer(&observer),
        Err(Ok(Error::ObserverAlreadyRegistered))
    );

    let case_id = sample_bytes(&env, 1);
    client.create_case(&case_id, &owner, &None, &sample_bytes(&env, 2), &200);

    // Repeated create_case
    assert_eq!(
        client.try_create_case(&case_id, &owner, &None, &sample_bytes(&env, 2), &200),
        Err(Ok(Error::CaseAlreadyExists))
    );

    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );

    // Repeated observation
    assert_eq!(
        client.try_record_observation(
            &observer,
            &case_id,
            &sample_bytes(&env, 3),
            &100,
            &sample_bytes(&env, 4),
        ),
        Err(Ok(Error::InvalidState))
    );

    client.record_match(&observer, &case_id);

    // Repeated match
    assert_eq!(
        client.try_record_match(&observer, &case_id),
        Err(Ok(Error::InvalidState))
    );

    // Repeated attestation
    client.submit_attestation(&case_id, &AttestationRole::Owner, &sample_bytes(&env, 10));
    assert_eq!(
        client.try_submit_attestation(&case_id, &AttestationRole::Owner, &sample_bytes(&env, 10)),
        Err(Ok(Error::AttestationAlreadyExists))
    );

    client.submit_attestation(
        &case_id,
        &AttestationRole::Observer,
        &sample_bytes(&env, 11),
    );
    client.finalize_case(&case_id);

    // Repeated finalization
    assert_eq!(
        client.try_finalize_case(&case_id),
        Err(Ok(Error::InvalidState))
    );
}

#[test]
fn test_boundary_repeated_conflicting_calls() {
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

    // Record Match then attempt Break
    client.record_match(&observer, &case_id);
    assert_eq!(
        client.try_record_break(&observer, &case_id, &BreakCode::DestinationMismatch),
        Err(Ok(Error::InvalidState))
    );

    // Attempt to open dispute on matched case
    assert_eq!(
        client.try_open_dispute(&owner, &case_id, &sample_bytes(&env, 30)),
        Err(Ok(Error::InvalidState))
    );
}

#[test]
fn test_boundary_already_finalized_case_rejects_all_mutations() {
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

    // Verify all endpoints reject on finalized case
    assert_eq!(
        client.try_record_observation(
            &observer,
            &case_id,
            &sample_bytes(&env, 5),
            &100,
            &sample_bytes(&env, 6),
        ),
        Err(Ok(Error::InvalidState))
    );
    assert_eq!(
        client.try_record_match(&observer, &case_id),
        Err(Ok(Error::InvalidState))
    );
    assert_eq!(
        client.try_record_break(&observer, &case_id, &BreakCode::LateSettlement),
        Err(Ok(Error::InvalidState))
    );
    assert_eq!(
        client.try_open_dispute(&owner, &case_id, &sample_bytes(&env, 30)),
        Err(Ok(Error::InvalidState))
    );
    assert_eq!(
        client.try_submit_resolution(&owner, &case_id, &sample_bytes(&env, 31)),
        Err(Ok(Error::InvalidState))
    );
    assert_eq!(
        client.try_finalize_case(&case_id),
        Err(Ok(Error::InvalidState))
    );
}

#[test]
fn test_boundary_already_resolved_dispute_rejects_further_resolution() {
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

    // Attempt resolution on already resolved case
    assert_eq!(
        client.try_submit_resolution(&owner, &case_id, &sample_bytes(&env, 32)),
        Err(Ok(Error::InvalidState))
    );
    assert_eq!(
        client.try_submit_resolution(&counterparty, &case_id, &sample_bytes(&env, 32)),
        Err(Ok(Error::InvalidState))
    );
}

// ---------------------------------------------------------------------------
// RELEASE-CANDIDATE SECURITY REGRESSION SUITE
// ---------------------------------------------------------------------------

#[test]
fn test_rc_security_authorization_boundaries() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let counterparty = Address::generate(&env);
    let observer = Address::generate(&env);
    let malicious = Address::generate(&env);

    client.add_observer(&observer);

    let case_id = sample_bytes(&env, 1);
    let terms = sample_bytes(&env, 2);
    client.create_case(&case_id, &owner, &Some(counterparty.clone()), &terms, &500);

    // Malicious caller cannot record observation
    assert_eq!(
        client.try_record_observation(
            &malicious,
            &case_id,
            &sample_bytes(&env, 3),
            &100,
            &sample_bytes(&env, 4),
        ),
        Err(Ok(Error::ObserverNotRegistered))
    );

    // Record legitimate observation
    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );

    // Malicious caller cannot record match or break
    assert_eq!(
        client.try_record_match(&malicious, &case_id),
        Err(Ok(Error::ObserverNotRegistered))
    );
    assert_eq!(
        client.try_record_break(&malicious, &case_id, &BreakCode::AmountMismatch),
        Err(Ok(Error::ObserverNotRegistered))
    );

    // Malicious caller cannot open dispute on Observed case
    assert_eq!(
        client.try_open_dispute(&malicious, &case_id, &sample_bytes(&env, 10)),
        Err(Ok(Error::Unauthorized))
    );
}

#[test]
fn test_rc_security_terminal_state_immutability() {
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
        &500,
    );
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
        &AttestationRole::Counterparty,
        &sample_bytes(&env, 11),
    );
    client.submit_attestation(
        &case_id,
        &AttestationRole::Observer,
        &sample_bytes(&env, 12),
    );

    // Finalize case into terminal state
    client.finalize_case(&case_id);
    assert_eq!(client.get_case(&case_id).status, CaseStatus::Finalized);

    // Verify all mutating endpoints are strictly rejected
    assert_eq!(
        client.try_record_observation(
            &observer,
            &case_id,
            &sample_bytes(&env, 5),
            &100,
            &sample_bytes(&env, 6),
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
        client.try_submit_resolution(&owner, &case_id, &sample_bytes(&env, 40)),
        Err(Ok(Error::InvalidState))
    );
    assert_eq!(
        client.try_finalize_case(&case_id),
        Err(Ok(Error::InvalidState))
    );

    // Stored case record remains completely intact
    let final_case = client.get_case(&case_id);
    assert_eq!(final_case.status, CaseStatus::Finalized);
    assert_eq!(final_case.decision, Decision::Matched);
    assert_eq!(final_case.finalized_at_ledger, Some(100));
}

#[test]
fn test_rc_security_duplicate_calls() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let observer = Address::generate(&env);

    // Duplicate observer registration
    client.add_observer(&observer);
    assert_eq!(
        client.try_add_observer(&observer),
        Err(Ok(Error::ObserverAlreadyRegistered))
    );

    let case_id = sample_bytes(&env, 1);
    let terms = sample_bytes(&env, 2);
    client.create_case(&case_id, &owner, &None, &terms, &500);

    // Duplicate case creation
    assert_eq!(
        client.try_create_case(&case_id, &owner, &None, &terms, &500),
        Err(Ok(Error::CaseAlreadyExists))
    );

    // Duplicate observation recording
    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );
    assert_eq!(
        client.try_record_observation(
            &observer,
            &case_id,
            &sample_bytes(&env, 5),
            &100,
            &sample_bytes(&env, 6),
        ),
        Err(Ok(Error::InvalidState))
    );

    // Duplicate match recording
    client.record_match(&observer, &case_id);
    assert_eq!(
        client.try_record_match(&observer, &case_id),
        Err(Ok(Error::InvalidState))
    );

    // Duplicate attestation
    client.submit_attestation(&case_id, &AttestationRole::Owner, &sample_bytes(&env, 10));
    assert_eq!(
        client.try_submit_attestation(&case_id, &AttestationRole::Owner, &sample_bytes(&env, 11)),
        Err(Ok(Error::AttestationAlreadyExists))
    );
}

#[test]
fn test_rc_security_commitment_integrity() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    let zero = zero_bytes(&env);
    let valid = sample_bytes(&env, 1);

    // Zero terms commitment rejected
    assert_eq!(
        client.try_create_case(&sample_bytes(&env, 2), &owner, &None, &zero, &500),
        Err(Ok(Error::InvalidCommitment))
    );

    let case_id = sample_bytes(&env, 3);
    client.create_case(&case_id, &owner, &None, &valid, &500);

    // Zero observation commitment rejected
    assert_eq!(
        client.try_record_observation(&observer, &case_id, &sample_bytes(&env, 4), &100, &zero),
        Err(Ok(Error::InvalidCommitment))
    );

    // Zero attestation commitment rejected
    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 4),
        &100,
        &sample_bytes(&env, 5),
    );
    client.record_match(&observer, &case_id);
    assert_eq!(
        client.try_submit_attestation(&case_id, &AttestationRole::Owner, &zero),
        Err(Ok(Error::InvalidCommitment))
    );
}

#[test]
fn test_rc_security_attestation_and_finalization_rules() {
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
        &500,
    );
    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );
    client.record_match(&observer, &case_id);

    // Premature finalization without attestations fails
    assert_eq!(
        client.try_finalize_case(&case_id),
        Err(Ok(Error::MissingRequiredAttestation))
    );

    // Finalization with only owner attestation fails
    client.submit_attestation(&case_id, &AttestationRole::Owner, &sample_bytes(&env, 10));
    assert_eq!(
        client.try_finalize_case(&case_id),
        Err(Ok(Error::ObserverQuorumNotMet))
    );

    // Finalization succeeds once observer attests
    client.submit_attestation(
        &case_id,
        &AttestationRole::Observer,
        &sample_bytes(&env, 11),
    );
    assert_eq!(client.finalize_case(&case_id), ());
    assert_eq!(client.get_case(&case_id).status, CaseStatus::Finalized);
}

#[test]
fn test_rc_security_dispute_and_resolution_rules() {
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
        &500,
    );
    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );
    client.record_break(&observer, &case_id, &BreakCode::AmountMismatch);

    // Non-party cannot open dispute
    let bystander = Address::generate(&env);
    assert_eq!(
        client.try_open_dispute(&bystander, &case_id, &sample_bytes(&env, 20)),
        Err(Ok(Error::Unauthorized))
    );

    // Legitimate dispute opening
    client.open_dispute(&owner, &case_id, &sample_bytes(&env, 20));
    assert_eq!(client.get_case(&case_id).status, CaseStatus::Disputed);

    // Mismatched resolution commitments keep status in Disputed
    let res_owner = sample_bytes(&env, 30);
    let res_cp = sample_bytes(&env, 31);
    client.submit_resolution(&owner, &case_id, &res_owner);
    client.submit_resolution(&counterparty, &case_id, &res_cp);
    assert_eq!(client.get_case(&case_id).status, CaseStatus::Disputed);
}

// ---------------------------------------------------------------------------
// MALFORMED-INPUT AND RESOURCE-BOUND TEST SUITE
// ---------------------------------------------------------------------------

#[test]
fn test_malformed_input_all_zero_and_all_ones_commitments() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    let zero = zero_bytes(&env);
    let all_ones = sample_bytes(&env, 0xff);

    // All zero terms commitment rejected
    assert_eq!(
        client.try_create_case(&sample_bytes(&env, 1), &owner, &None, &zero, &500),
        Err(Ok(Error::InvalidCommitment))
    );

    // All ones terms commitment accepted
    let case_id = sample_bytes(&env, 2);
    assert_eq!(
        client.create_case(&case_id, &owner, &None, &all_ones, &500),
        ()
    );

    // All zero observation commitment rejected
    assert_eq!(
        client.try_record_observation(&observer, &case_id, &sample_bytes(&env, 3), &100, &zero),
        Err(Ok(Error::InvalidCommitment))
    );

    // All ones observation commitment accepted
    assert_eq!(
        client.record_observation(&observer, &case_id, &all_ones, &100, &all_ones),
        ()
    );
}

#[test]
fn test_malformed_input_extreme_ledger_boundaries() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    env.ledger().set_sequence_number(1_000);

    let case_id = sample_bytes(&env, 10);
    let terms = sample_bytes(&env, 20);

    // Expiration at or before current ledger rejected
    assert_eq!(
        client.try_create_case(&case_id, &owner, &None, &terms, &1_000),
        Err(Ok(Error::InvalidExpiration))
    );
    assert_eq!(
        client.try_create_case(&case_id, &owner, &None, &terms, &999),
        Err(Ok(Error::InvalidExpiration))
    );

    // Maximum ledger sequence u32::MAX accepted
    client.create_case(&case_id, &owner, &None, &terms, &u32::MAX);

    // Observation at ledger 0 rejected
    assert_eq!(
        client.try_record_observation(
            &observer,
            &case_id,
            &sample_bytes(&env, 1),
            &0,
            &sample_bytes(&env, 2)
        ),
        Err(Ok(Error::InvalidLedger))
    );

    // Observation in the future (> current ledger) rejected
    assert_eq!(
        client.try_record_observation(
            &observer,
            &case_id,
            &sample_bytes(&env, 1),
            &1_001,
            &sample_bytes(&env, 2)
        ),
        Err(Ok(Error::InvalidLedger))
    );

    // Observation at exact current ledger accepted
    assert_eq!(
        client.record_observation(
            &observer,
            &case_id,
            &sample_bytes(&env, 1),
            &1_000,
            &sample_bytes(&env, 2)
        ),
        ()
    );
}

#[test]
fn test_malformed_input_invalid_state_transition_combinations() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    let case_id = sample_bytes(&env, 50);
    client.create_case(&case_id, &owner, &None, &sample_bytes(&env, 51), &500);

    // From Open: cannot record match, break, dispute, resolution, finalize directly
    assert_eq!(
        client.try_record_match(&observer, &case_id),
        Err(Ok(Error::InvalidState))
    );
    assert_eq!(
        client.try_record_break(&observer, &case_id, &BreakCode::AmountMismatch),
        Err(Ok(Error::InvalidState))
    );
    assert_eq!(
        client.try_open_dispute(&owner, &case_id, &sample_bytes(&env, 52)),
        Err(Ok(Error::InvalidState))
    );
    assert_eq!(
        client.try_submit_resolution(&owner, &case_id, &sample_bytes(&env, 53)),
        Err(Ok(Error::InvalidState))
    );
    assert_eq!(
        client.try_finalize_case(&case_id),
        Err(Ok(Error::InvalidState))
    );
}

#[test]
fn test_resource_bound_multiple_case_lifecycle_and_reads() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    // Create 10 distinct settlement cases
    for i in 1..=10 {
        let case_id = sample_bytes(&env, i);
        let terms = sample_bytes(&env, i + 100);
        client.create_case(&case_id, &owner, &None, &terms, &5_000);

        let tx_hash = sample_bytes(&env, i + 10);
        let obs_comm = sample_bytes(&env, i + 20);
        client.record_observation(&observer, &case_id, &tx_hash, &100, &obs_comm);
        client.record_match(&observer, &case_id);
        client.submit_attestation(
            &case_id,
            &AttestationRole::Owner,
            &sample_bytes(&env, i + 30),
        );
        client.submit_attestation(
            &case_id,
            &AttestationRole::Observer,
            &sample_bytes(&env, i + 40),
        );
        client.finalize_case(&case_id);

        let final_case = client.get_case(&case_id);
        assert_eq!(final_case.status, CaseStatus::Finalized);
    }
}

// ---------------------------------------------------------------------------
// OBSERVER QUORUM TESTS
// ---------------------------------------------------------------------------

#[test]
fn test_observer_quorum_configuration() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let case_id = sample_bytes(&env, 1);

    client.create_case(&case_id, &owner, &None, &sample_bytes(&env, 2), &500);

    // Default quorum is 1
    assert_eq!(client.get_case_quorum(&case_id), 1);
    let case = client.get_case(&case_id);
    assert_eq!(case.observer_quorum, 1);

    // Setting quorum to 0 fails
    assert_eq!(
        client.try_set_case_quorum(&case_id, &0),
        Err(Ok(Error::InvalidObserverQuorum))
    );

    // Setting quorum to 3 succeeds
    client.set_case_quorum(&case_id, &3);
    assert_eq!(client.get_case_quorum(&case_id), 3);
    let updated = client.get_case(&case_id);
    assert_eq!(updated.observer_quorum, 3);
}

#[test]
fn test_observer_quorum_enforcement_and_multi_observer_finalization() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let obs1 = Address::generate(&env);
    let obs2 = Address::generate(&env);
    let obs3 = Address::generate(&env);

    client.add_observer(&obs1);
    client.add_observer(&obs2);
    client.add_observer(&obs3);

    let case_id = sample_bytes(&env, 1);
    client.create_case(&case_id, &owner, &None, &sample_bytes(&env, 2), &500);
    client.set_case_quorum(&case_id, &3);

    // Record observation and match with obs1
    client.record_observation(
        &obs1,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );
    client.record_match(&obs1, &case_id);

    // Owner attests
    client.submit_attestation(&case_id, &AttestationRole::Owner, &sample_bytes(&env, 10));

    // obs1 attests -> 1 observer (insufficient for quorum 3)
    client.submit_attestation(
        &case_id,
        &AttestationRole::Observer,
        &sample_bytes(&env, 11),
    );
    assert_eq!(
        client.try_finalize_case(&case_id),
        Err(Ok(Error::ObserverQuorumNotMet))
    );

    // obs2 attests -> 2 observers (still insufficient for quorum 3)
    client.submit_observer_attestation(&case_id, &obs2, &sample_bytes(&env, 12));
    assert_eq!(
        client.try_finalize_case(&case_id),
        Err(Ok(Error::ObserverQuorumNotMet))
    );

    // duplicate attestation from obs2 fails
    assert_eq!(
        client.try_submit_observer_attestation(&case_id, &obs2, &sample_bytes(&env, 13)),
        Err(Ok(Error::AttestationAlreadyExists))
    );

    // obs3 attests -> 3 distinct registered observers
    client.submit_observer_attestation(&case_id, &obs3, &sample_bytes(&env, 14));

    // Finalization now succeeds!
    client.finalize_case(&case_id);
    let final_case = client.get_case(&case_id);
    assert_eq!(final_case.status, CaseStatus::Finalized);
}

#[test]
fn test_observer_quorum_1_legacy_path() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    let case_id = sample_bytes(&env, 1);
    client.create_case(&case_id, &owner, &None, &sample_bytes(&env, 2), &500);

    // Default quorum is 1
    assert_eq!(client.get_case_quorum(&case_id), 1);

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
    let final_case = client.get_case(&case_id);
    assert_eq!(final_case.status, CaseStatus::Finalized);
    assert_eq!(final_case.observer_quorum, 1);
}

#[test]
fn test_observer_quorum_2_success_and_failure() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let obs1 = Address::generate(&env);
    let obs2 = Address::generate(&env);

    client.add_observer(&obs1);
    client.add_observer(&obs2);

    let case_id = sample_bytes(&env, 1);
    client.create_case(&case_id, &owner, &None, &sample_bytes(&env, 2), &500);
    client.set_case_quorum(&case_id, &2);

    client.record_observation(
        &obs1,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );
    client.record_match(&obs1, &case_id);

    // Owner attests
    client.submit_attestation(&case_id, &AttestationRole::Owner, &sample_bytes(&env, 10));

    // obs1 attests -> 1 observer (insufficient for quorum 2)
    client.submit_attestation(
        &case_id,
        &AttestationRole::Observer,
        &sample_bytes(&env, 11),
    );
    assert_eq!(
        client.try_finalize_case(&case_id),
        Err(Ok(Error::ObserverQuorumNotMet))
    );

    // obs2 attests via submit_observer_attestation
    client.submit_observer_attestation(&case_id, &obs2, &sample_bytes(&env, 12));

    // Quorum 2 is now met, finalization succeeds
    client.finalize_case(&case_id);
    let final_case = client.get_case(&case_id);
    assert_eq!(final_case.status, CaseStatus::Finalized);
}

#[test]
fn test_observer_quorum_original_observer_not_mandatory_for_finalization() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let original_obs = Address::generate(&env);
    let obs_a = Address::generate(&env);
    let obs_b = Address::generate(&env);

    client.add_observer(&original_obs);
    client.add_observer(&obs_a);
    client.add_observer(&obs_b);

    let case_id = sample_bytes(&env, 1);
    client.create_case(&case_id, &owner, &None, &sample_bytes(&env, 2), &500);
    client.set_case_quorum(&case_id, &2);

    // original_obs records observation and match, but NEVER attests
    client.record_observation(
        &original_obs,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );
    client.record_match(&original_obs, &case_id);

    // Owner attests
    client.submit_attestation(&case_id, &AttestationRole::Owner, &sample_bytes(&env, 10));

    // Two distinct observers (obs_a, obs_b) attest; original_obs does NOT attest
    client.submit_observer_attestation(&case_id, &obs_a, &sample_bytes(&env, 20));
    client.submit_observer_attestation(&case_id, &obs_b, &sample_bytes(&env, 21));

    // Pure M-of-N finalization succeeds!
    client.finalize_case(&case_id);
    let final_case = client.get_case(&case_id);
    assert_eq!(final_case.status, CaseStatus::Finalized);
}

#[test]
fn test_observer_quorum_duplicate_and_repeated_attestations() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let obs1 = Address::generate(&env);
    client.add_observer(&obs1);

    let case_id = sample_bytes(&env, 1);
    client.create_case(&case_id, &owner, &None, &sample_bytes(&env, 2), &500);
    client.set_case_quorum(&case_id, &3);

    client.record_observation(
        &obs1,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );
    client.record_match(&obs1, &case_id);

    client.submit_attestation(&case_id, &AttestationRole::Owner, &sample_bytes(&env, 10));
    client.submit_attestation(
        &case_id,
        &AttestationRole::Observer,
        &sample_bytes(&env, 11),
    );

    // obs1 trying to submit duplicate attestation fails
    assert_eq!(
        client.try_submit_observer_attestation(&case_id, &obs1, &sample_bytes(&env, 12)),
        Err(Ok(Error::AttestationAlreadyExists))
    );

    // Any one observer CANNOT finalize a quorum-3 case
    assert_eq!(
        client.try_finalize_case(&case_id),
        Err(Ok(Error::ObserverQuorumNotMet))
    );
}

#[test]
fn test_observer_quorum_preserves_historical_attestation_after_revocation() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let obs1 = Address::generate(&env);
    let obs2 = Address::generate(&env);

    client.add_observer(&obs1);
    client.add_observer(&obs2);

    let case_id = sample_bytes(&env, 1);
    client.create_case(&case_id, &owner, &None, &sample_bytes(&env, 2), &500);
    client.set_case_quorum(&case_id, &2);

    client.record_observation(
        &obs1,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );
    client.record_match(&obs1, &case_id);

    client.submit_attestation(&case_id, &AttestationRole::Owner, &sample_bytes(&env, 10));
    client.submit_attestation(
        &case_id,
        &AttestationRole::Observer,
        &sample_bytes(&env, 11),
    );
    client.submit_observer_attestation(&case_id, &obs2, &sample_bytes(&env, 12));

    // Revoke obs2 after its attestation has already been recorded
    client.remove_observer(&obs2);

    // Finalization succeeds because obs2 was authorized at attestation submission time
    client.finalize_case(&case_id);
    let final_case = client.get_case(&case_id);
    assert_eq!(final_case.status, CaseStatus::Finalized);

    // Newly revoked observer cannot submit a new attestation
    let case2_id = sample_bytes(&env, 99);
    client.create_case(&case2_id, &owner, &None, &sample_bytes(&env, 2), &500);
    assert_eq!(
        client.try_submit_observer_attestation(&case2_id, &obs2, &sample_bytes(&env, 13)),
        Err(Ok(Error::ObserverNotRegistered))
    );

    // Unregistered observer cannot submit attestation
    let obs3 = Address::generate(&env);
    assert_eq!(
        client.try_submit_observer_attestation(&case2_id, &obs3, &sample_bytes(&env, 14)),
        Err(Ok(Error::ObserverNotRegistered))
    );
}

#[test]
fn test_role_separation_owner_and_counterparty_cannot_attest_as_observer() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let counterparty = Address::generate(&env);
    let obs = Address::generate(&env);

    client.add_observer(&obs);
    // Adversarial: admin registers owner and counterparty as observers
    client.add_observer(&owner);
    client.add_observer(&counterparty);

    let case_id = sample_bytes(&env, 1);
    client.create_case(
        &case_id,
        &owner,
        &Some(counterparty.clone()),
        &sample_bytes(&env, 2),
        &500,
    );

    // Owner cannot submit observer attestation for their own case
    assert_eq!(
        client.try_submit_observer_attestation(&case_id, &owner, &sample_bytes(&env, 10)),
        Err(Ok(Error::Unauthorized))
    );

    // Counterparty cannot submit observer attestation for their case
    assert_eq!(
        client.try_submit_observer_attestation(&case_id, &counterparty, &sample_bytes(&env, 11)),
        Err(Ok(Error::Unauthorized))
    );
}

// ---------------------------------------------------------------------------
// DISPUTE EXPIRATION TESTS
// ---------------------------------------------------------------------------

#[test]
fn test_dispute_expiration_lifecycle() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let counterparty = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    let case_id = sample_bytes(&env, 1);
    client.create_case(
        &case_id,
        &owner,
        &Some(counterparty),
        &sample_bytes(&env, 2),
        &50_000,
    );

    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );
    client.record_break(&observer, &case_id, &BreakCode::AmountMismatch);

    // Open dispute with TTL = 200 ledgers
    client.open_dispute_with_ttl(&owner, &case_id, &sample_bytes(&env, 20), &200);
    let case = client.get_case(&case_id);
    assert_eq!(case.status, CaseStatus::Disputed);
    assert_eq!(case.dispute_expires_at_ledger, Some(300)); // current ledger 100 + 200
    assert_eq!(client.get_dispute_expiration(&case_id), Some(300));

    // Cannot expire before ledger 300
    assert_eq!(
        client.try_expire_dispute(&case_id),
        Err(Ok(Error::DisputeNotExpired))
    );

    // Advance ledger to 301
    env.ledger().set_sequence_number(301);

    // Expire dispute
    client.expire_dispute(&case_id);

    // Case is now back in Break state, expiration record removed
    let expired_case = client.get_case(&case_id);
    assert_eq!(expired_case.status, CaseStatus::Break);
    assert_eq!(expired_case.dispute_expires_at_ledger, None);
    assert_eq!(client.get_dispute_expiration(&case_id), None);

    // Can reopen dispute from Break
    client.open_dispute(&owner, &case_id, &sample_bytes(&env, 21));
    let reopened = client.get_case(&case_id);
    assert_eq!(reopened.status, CaseStatus::Disputed);
}

#[test]
fn test_dispute_resolution_clears_expiration() {
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
        &50_000,
    );

    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );
    client.record_break(&observer, &case_id, &BreakCode::AmountMismatch);

    client.open_dispute(&owner, &case_id, &sample_bytes(&env, 20));
    assert!(client.get_dispute_expiration(&case_id).is_some());

    // Submit matching resolutions
    let res_comm = sample_bytes(&env, 30);
    client.submit_resolution(&owner, &case_id, &res_comm);
    client.submit_resolution(&counterparty, &case_id, &res_comm);

    // Case is resolved, dispute expiration is cleared
    let case = client.get_case(&case_id);
    assert_eq!(case.status, CaseStatus::Resolved);
    assert_eq!(case.dispute_expires_at_ledger, None);
    assert_eq!(client.get_dispute_expiration(&case_id), None);
}

#[test]
fn test_dispute_cannot_open_without_counterparty() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    let case_id = sample_bytes(&env, 1);
    client.create_case(&case_id, &owner, &None, &sample_bytes(&env, 2), &500);

    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );
    client.record_break(&observer, &case_id, &BreakCode::AmountMismatch);

    // open_dispute fails with CounterpartyRequired
    assert_eq!(
        client.try_open_dispute(&owner, &case_id, &sample_bytes(&env, 10)),
        Err(Ok(Error::CounterpartyRequired))
    );

    // open_dispute_with_ttl fails with CounterpartyRequired
    assert_eq!(
        client.try_open_dispute_with_ttl(&owner, &case_id, &sample_bytes(&env, 10), &200),
        Err(Ok(Error::CounterpartyRequired))
    );
}

#[test]
fn test_dispute_expiry_timing_boundaries() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let counterparty = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    let case_id = sample_bytes(&env, 1);
    client.create_case(
        &case_id,
        &owner,
        &Some(counterparty),
        &sample_bytes(&env, 2),
        &50_000,
    );

    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );
    client.record_break(&observer, &case_id, &BreakCode::AmountMismatch);

    // Ledger is 100. Open dispute with TTL = 200 ledgers -> expires at 300
    client.open_dispute_with_ttl(&owner, &case_id, &sample_bytes(&env, 20), &200);
    assert_eq!(client.get_dispute_expiration(&case_id), Some(300));

    // Immediately before TTL (ledger 299): rejects expiry
    env.ledger().set_sequence_number(299);
    assert_eq!(
        client.try_expire_dispute(&case_id),
        Err(Ok(Error::DisputeNotExpired))
    );

    // Exactly at TTL (ledger 300): succeeds!
    env.ledger().set_sequence_number(300);
    client.expire_dispute(&case_id);
    let case = client.get_case(&case_id);
    assert_eq!(case.status, CaseStatus::Break);
    assert_eq!(case.dispute_expires_at_ledger, None);

    // Reopen dispute with TTL = 150 at ledger 300 -> expires at 450
    client.open_dispute_with_ttl(&owner, &case_id, &sample_bytes(&env, 21), &150);
    assert_eq!(client.get_dispute_expiration(&case_id), Some(450));

    // After TTL (ledger 500): succeeds!
    env.ledger().set_sequence_number(500);
    client.expire_dispute(&case_id);
    let case2 = client.get_case(&case_id);
    assert_eq!(case2.status, CaseStatus::Break);
}

#[test]
fn test_dispute_expiry_state_validation_and_idempotence() {
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
        &50_000,
    );

    // Calling expire_dispute on Open state fails with InvalidState
    assert_eq!(
        client.try_expire_dispute(&case_id),
        Err(Ok(Error::InvalidState))
    );

    // Non-existent case fails with NotFound
    let bogus = sample_bytes(&env, 99);
    assert_eq!(client.try_expire_dispute(&bogus), Err(Ok(Error::NotFound)));

    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );
    client.record_break(&observer, &case_id, &BreakCode::AmountMismatch);

    client.open_dispute_with_ttl(&owner, &case_id, &sample_bytes(&env, 20), &200);
    env.ledger().set_sequence_number(301);

    // First expiry succeeds
    client.expire_dispute(&case_id);
    assert_eq!(client.get_case(&case_id).status, CaseStatus::Break);

    // Repeated/duplicate expiry invocation fails with InvalidState (idempotent through state validation)
    assert_eq!(
        client.try_expire_dispute(&case_id),
        Err(Ok(Error::InvalidState))
    );
}

#[test]
fn test_dispute_already_expired_rejects_resolution_submission() {
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
        &50_000,
    );

    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );
    client.record_break(&observer, &case_id, &BreakCode::AmountMismatch);

    // Dispute expires at ledger 300
    client.open_dispute_with_ttl(&owner, &case_id, &sample_bytes(&env, 20), &200);

    // Advance ledger to 300 (expired)
    env.ledger().set_sequence_number(300);

    // Submitting resolution after expiration ledger fails with DisputeAlreadyExpired
    assert_eq!(
        client.try_submit_resolution(&owner, &case_id, &sample_bytes(&env, 30)),
        Err(Ok(Error::DisputeAlreadyExpired))
    );
}

#[test]
fn test_finalized_case_rejects_attestations_and_quorum_mutation() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let observer = Address::generate(&env);
    let observer2 = Address::generate(&env);
    client.add_observer(&observer);
    client.add_observer(&observer2);

    let case_id = sample_bytes(&env, 1);
    client.create_case(&case_id, &owner, &None, &sample_bytes(&env, 2), &500);

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
    assert_eq!(client.get_case(&case_id).status, CaseStatus::Finalized);

    // All mutations rejected on finalized case
    assert_eq!(
        client.try_set_case_quorum(&case_id, &2),
        Err(Ok(Error::InvalidState))
    );
    assert_eq!(
        client.try_submit_attestation(&case_id, &AttestationRole::Owner, &sample_bytes(&env, 20)),
        Err(Ok(Error::InvalidState))
    );
    assert_eq!(
        client.try_submit_observer_attestation(&case_id, &observer2, &sample_bytes(&env, 21)),
        Err(Ok(Error::InvalidState))
    );
    assert_eq!(
        client.try_expire_dispute(&case_id),
        Err(Ok(Error::InvalidState))
    );
}

#[test]
fn test_event_completeness_quorum_and_dispute_expired() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let counterparty = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    let case_id = sample_bytes(&env, 1);
    client.create_case(
        &case_id,
        &owner,
        &Some(counterparty),
        &sample_bytes(&env, 2),
        &50_000,
    );

    // Event on set_case_quorum (CaseQuorumSet emitted)
    client.set_case_quorum(&case_id, &3);
    assert_eq!(env.events().all().events().len(), 1);

    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );
    client.record_break(&observer, &case_id, &BreakCode::AmountMismatch);

    client.open_dispute_with_ttl(&owner, &case_id, &sample_bytes(&env, 20), &200);
    env.ledger().set_sequence_number(301);

    // Event on expire_dispute (DisputeExpired emitted)
    client.expire_dispute(&case_id);
    assert_eq!(env.events().all().events().len(), 1);
}

#[test]
fn test_quorum_and_dispute_ttl_bounds_and_invariants() {
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
        &1_000_000,
    );

    // 1. set_case_quorum rejects 0 with InvalidObserverQuorum
    assert_eq!(
        client.try_set_case_quorum(&case_id, &0),
        Err(Ok(Error::InvalidObserverQuorum))
    );

    // 2. set_case_quorum rejects > MAX_OBSERVER_QUORUM (10) with ObserverQuorumExceeded
    assert_eq!(
        client.try_set_case_quorum(&case_id, &(crate::storage::MAX_OBSERVER_QUORUM + 1)),
        Err(Ok(Error::ObserverQuorumExceeded))
    );

    // 3. set_case_quorum accepts MAX_OBSERVER_QUORUM (10)
    assert_eq!(
        client.try_set_case_quorum(&case_id, &crate::storage::MAX_OBSERVER_QUORUM),
        Ok(Ok(()))
    );
    assert_eq!(
        client.get_case_quorum(&case_id),
        crate::storage::MAX_OBSERVER_QUORUM
    );

    // Transition to Observed then Break
    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 3),
        &100,
        &sample_bytes(&env, 4),
    );

    // 4. Test observer limit per case: MAX_OBSERVERS_PER_CASE (16)
    // Register 16 observers and submit attestations
    for i in 1..=crate::storage::MAX_OBSERVERS_PER_CASE {
        let obs = Address::generate(&env);
        client.add_observer(&obs);
        assert_eq!(
            client.try_submit_observer_attestation(
                &case_id,
                &obs,
                &sample_bytes(&env, (100 + i) as u8)
            ),
            Ok(Ok(()))
        );
    }
    assert_eq!(
        client.get_attested_observers(&case_id).len(),
        crate::storage::MAX_OBSERVERS_PER_CASE
    );

    // 5. 17th observer attestation exceeds limit -> ObserverLimitExceeded
    let obs_17 = Address::generate(&env);
    client.add_observer(&obs_17);
    assert_eq!(
        client.try_submit_observer_attestation(&case_id, &obs_17, &sample_bytes(&env, 200)),
        Err(Ok(Error::ObserverLimitExceeded))
    );

    // Move to Break
    client.record_break(&observer, &case_id, &BreakCode::AmountMismatch);

    // 6. open_dispute_with_ttl rejects < MIN_DISPUTE_TTL_LEDGERS
    assert_eq!(
        client.try_open_dispute_with_ttl(
            &owner,
            &case_id,
            &sample_bytes(&env, 50),
            &(crate::storage::MIN_DISPUTE_TTL_LEDGERS - 1)
        ),
        Err(Ok(Error::InvalidExpiration))
    );

    // 7. open_dispute_with_ttl rejects > MAX_DISPUTE_TTL_LEDGERS
    assert_eq!(
        client.try_open_dispute_with_ttl(
            &owner,
            &case_id,
            &sample_bytes(&env, 50),
            &(crate::storage::MAX_DISPUTE_TTL_LEDGERS + 1)
        ),
        Err(Ok(Error::InvalidExpiration))
    );

    // 8. open_dispute_with_ttl accepts MIN_DISPUTE_TTL_LEDGERS
    let cur_ledger = env.ledger().sequence();
    assert_eq!(
        client.try_open_dispute_with_ttl(
            &owner,
            &case_id,
            &sample_bytes(&env, 50),
            &crate::storage::MIN_DISPUTE_TTL_LEDGERS
        ),
        Ok(Ok(()))
    );
    assert_eq!(
        client.get_dispute_expiration(&case_id),
        Some(cur_ledger + crate::storage::MIN_DISPUTE_TTL_LEDGERS)
    );
}

#[test]
fn test_negative_fuzz_case_creation_ledger_boundaries() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let counterparty = Address::generate(&env);
    let case_id_base = 200u8;

    env.ledger().set_sequence_number(5000);
    let cur = env.ledger().sequence();
    assert_eq!(cur, 5000);

    // 1. expires_at_ledger = 0 fails with InvalidExpiration
    let cid0 = sample_bytes(&env, case_id_base);
    assert_eq!(
        client.try_create_case(
            &cid0,
            &owner,
            &Some(counterparty.clone()),
            &sample_bytes(&env, 1),
            &0
        ),
        Err(Ok(Error::InvalidExpiration))
    );

    // 2. expires_at_ledger = current_ledger (5000) fails with InvalidExpiration
    let cid1 = sample_bytes(&env, case_id_base + 1);
    assert_eq!(
        client.try_create_case(
            &cid1,
            &owner,
            &Some(counterparty.clone()),
            &sample_bytes(&env, 1),
            &cur
        ),
        Err(Ok(Error::InvalidExpiration))
    );

    // 3. expires_at_ledger = current_ledger - 1 (4999) fails with InvalidExpiration
    let cid2 = sample_bytes(&env, case_id_base + 2);
    assert_eq!(
        client.try_create_case(
            &cid2,
            &owner,
            &Some(counterparty.clone()),
            &sample_bytes(&env, 1),
            &(cur - 1)
        ),
        Err(Ok(Error::InvalidExpiration))
    );

    // 4. expires_at_ledger = current_ledger + 1 (5001) succeeds
    let cid3 = sample_bytes(&env, case_id_base + 3);
    assert_eq!(
        client.try_create_case(
            &cid3,
            &owner,
            &Some(counterparty.clone()),
            &sample_bytes(&env, 1),
            &(cur + 1)
        ),
        Ok(Ok(()))
    );
    let c3 = client.get_case(&cid3);
    assert_eq!(c3.expires_at_ledger, 5001);
    assert_eq!(c3.status, CaseStatus::Open);

    // 5. expires_at_ledger = u32::MAX succeeds as valid distant boundary
    let cid4 = sample_bytes(&env, case_id_base + 4);
    assert_eq!(
        client.try_create_case(
            &cid4,
            &owner,
            &Some(counterparty.clone()),
            &sample_bytes(&env, 1),
            &u32::MAX
        ),
        Ok(Ok(()))
    );
    let c4 = client.get_case(&cid4);
    assert_eq!(c4.expires_at_ledger, u32::MAX);
    assert_eq!(c4.status, CaseStatus::Open);
}

#[test]
fn test_negative_fuzz_observation_ledger_boundaries() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    env.ledger().set_sequence_number(10000);
    let cur = env.ledger().sequence();

    let case_id = sample_bytes(&env, 210);
    client.create_case(
        &case_id,
        &owner,
        &None,
        &sample_bytes(&env, 1),
        &(cur + 1000),
    );

    // 1. observed_ledger = 0 fails with InvalidLedger
    assert_eq!(
        client.try_record_observation(
            &observer,
            &case_id,
            &sample_bytes(&env, 10),
            &0,
            &sample_bytes(&env, 20)
        ),
        Err(Ok(Error::InvalidLedger))
    );

    // 2. observed_ledger = cur + 1 (future ledger) fails with InvalidLedger
    assert_eq!(
        client.try_record_observation(
            &observer,
            &case_id,
            &sample_bytes(&env, 10),
            &(cur + 1),
            &sample_bytes(&env, 20)
        ),
        Err(Ok(Error::InvalidLedger))
    );

    // 3. observed_ledger = u32::MAX (future ledger boundary) fails with InvalidLedger
    assert_eq!(
        client.try_record_observation(
            &observer,
            &case_id,
            &sample_bytes(&env, 10),
            &u32::MAX,
            &sample_bytes(&env, 20)
        ),
        Err(Ok(Error::InvalidLedger))
    );

    // State preservation: case is still Open, no observation recorded
    let case = client.get_case(&case_id);
    assert_eq!(case.status, CaseStatus::Open);
    assert_eq!(case.observation, crate::types::Observation::None);

    // 4. observed_ledger = cur succeeds
    assert_eq!(
        client.try_record_observation(
            &observer,
            &case_id,
            &sample_bytes(&env, 10),
            &cur,
            &sample_bytes(&env, 20)
        ),
        Ok(Ok(()))
    );
    let case_after = client.get_case(&case_id);
    assert_eq!(case_after.status, CaseStatus::Observed);
}

#[test]
fn test_negative_fuzz_dispute_ttl_extreme_boundaries() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let counterparty = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    env.ledger().set_sequence_number(5000);
    let cur = env.ledger().sequence();

    let case_id = sample_bytes(&env, 220);
    client.create_case(
        &case_id,
        &owner,
        &Some(counterparty.clone()),
        &sample_bytes(&env, 1),
        &(cur + 5000),
    );
    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 10),
        &cur,
        &sample_bytes(&env, 20),
    );
    client.record_break(&observer, &case_id, &BreakCode::AmountMismatch);

    // 1. TTL = 0 fails
    assert_eq!(
        client.try_open_dispute_with_ttl(&owner, &case_id, &sample_bytes(&env, 30), &0),
        Err(Ok(Error::InvalidExpiration))
    );

    // 2. TTL = 1 fails (below MIN_DISPUTE_TTL_LEDGERS)
    assert_eq!(
        client.try_open_dispute_with_ttl(&owner, &case_id, &sample_bytes(&env, 30), &1),
        Err(Ok(Error::InvalidExpiration))
    );

    // 3. TTL = u32::MAX fails (exceeds MAX_DISPUTE_TTL_LEDGERS)
    assert_eq!(
        client.try_open_dispute_with_ttl(&owner, &case_id, &sample_bytes(&env, 30), &u32::MAX),
        Err(Ok(Error::InvalidExpiration))
    );

    // 4. TTL = u32::MAX - 1000 fails
    assert_eq!(
        client.try_open_dispute_with_ttl(
            &owner,
            &case_id,
            &sample_bytes(&env, 30),
            &(u32::MAX - 1000)
        ),
        Err(Ok(Error::InvalidExpiration))
    );

    // State preservation: case remains in Break status
    let case = client.get_case(&case_id);
    assert_eq!(case.status, CaseStatus::Break);
    assert_eq!(client.get_dispute_expiration(&case_id), None);
}

#[test]
fn test_negative_fuzz_dispute_expiration_sequence_drift() {
    let (env, _admin, client) = create_test_env();
    let owner = Address::generate(&env);
    let counterparty = Address::generate(&env);
    let observer = Address::generate(&env);
    client.add_observer(&observer);

    env.ledger().set_sequence_number(5000);
    let start_ledger = env.ledger().sequence();

    let case_id = sample_bytes(&env, 230);
    client.create_case(
        &case_id,
        &owner,
        &Some(counterparty.clone()),
        &sample_bytes(&env, 1),
        &(start_ledger + 5000),
    );
    client.record_observation(
        &observer,
        &case_id,
        &sample_bytes(&env, 10),
        &start_ledger,
        &sample_bytes(&env, 20),
    );
    client.record_break(&observer, &case_id, &BreakCode::AmountMismatch);

    let ttl = crate::storage::MIN_DISPUTE_TTL_LEDGERS;
    client.open_dispute_with_ttl(&owner, &case_id, &sample_bytes(&env, 30), &ttl);
    let exp_ledger = client.get_dispute_expiration(&case_id).unwrap();
    assert_eq!(exp_ledger, start_ledger + ttl);

    // 1. Premature expiration at exp_ledger - 1 fails with DisputeNotExpired
    env.ledger().set_sequence_number(exp_ledger - 1);
    assert_eq!(
        client.try_expire_dispute(&case_id),
        Err(Ok(Error::DisputeNotExpired))
    );
    let case_premature = client.get_case(&case_id);
    assert_eq!(case_premature.status, CaseStatus::Disputed);

    // 2. Artificial drift backwards to ledger 100 fails
    env.ledger().set_sequence_number(100);
    assert_eq!(
        client.try_expire_dispute(&case_id),
        Err(Ok(Error::DisputeNotExpired))
    );
    assert_eq!(client.get_case(&case_id).status, CaseStatus::Disputed);

    // 3. Advancing ledger to exact expiration sequence satisfies TTL
    env.ledger().set_sequence_number(exp_ledger);
    assert_eq!(client.try_expire_dispute(&case_id), Ok(Ok(())));
    let case_expired = client.get_case(&case_id);
    assert_eq!(case_expired.status, CaseStatus::Break);
    assert_eq!(client.get_dispute_expiration(&case_id), None);

    // 4. Repeated expiration call rejected as case is now in Break
    assert_eq!(
        client.try_expire_dispute(&case_id),
        Err(Ok(Error::InvalidState))
    );
}
