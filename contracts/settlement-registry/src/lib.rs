//! # SettlementRegistry Contract
//!
//! On-chain settlement evidence, observation registry, and reconciliation state machine for StellarClear.
//!
//! ## Security Invariants & Role Permissions
//! - **Case Creation**: Only authenticated `owner` with non-zero 32-byte `case_id`, non-zero `terms_commitment`, future expiration, and optional distinct `counterparty`.
//! - **Observer Registration**: Only contract `admin` can register or revoke observer addresses.
//! - **Observation Recording**: Only active, registered `observer` on `Open` cases with non-zero `tx_hash` and `observation_commitment`.
//! - **Reconciliation Decisions**: Only registered `observer` on `Observed` cases to record `Matched` or `Break` with standardized `BreakCode`.
//! - **Cryptographic Attestations**:
//!   - `Owner`: requires `owner.require_auth()`
//!   - `Counterparty`: requires `counterparty.require_auth()`
//!   - `Observer`: requires active registered `observer.require_auth()` matching case observer.
//!   - All attestations require non-zero 32-byte commitments.
//! - **Dispute Initiation**: Only `owner` or `counterparty` on `Break` cases with non-zero `dispute_commitment`.
//! - **Dispute Resolution**: `owner` and `counterparty` must independently submit identical non-zero resolution commitments to transition to `Resolved`.
//! - **Finalization**: Only `owner` on `Matched` cases (Owner + Observer attestations) or `Resolved` cases (Owner + Counterparty + Observer attestations).
//! - **Terminal Immutability**: Finalized cases are permanently immutable with no mutation methods or admin overrides.
//!
//! ## Security Assumptions & Disclaimer
//! - SHA-256 commitments represent deterministic off-chain pre-image calculations.
//! - Observers are trusted to accurately report on-chain ledger activity.
//! - **UNAUDITED SOFTWARE**: Prototype reference implementation under active development. Not audited for production value transfer.

#![no_std]

pub mod auth;
pub mod errors;
pub mod events;
pub mod storage;
pub mod types;

#[cfg(test)]
mod test;

use auth::{
    require_admin_auth, require_non_zero_commitment, require_observer_auth,
    validate_attestation_commitment, validate_case_identity, validate_observation_evidence,
    validate_state_transition, validate_terms_commitment,
};
use errors::Error;
use events::{
    emit_attestation_submitted, emit_case_broken, emit_case_created, emit_case_finalized,
    emit_case_matched, emit_case_quorum_set, emit_dispute_expired, emit_dispute_opened,
    emit_dispute_resolved, emit_observation_recorded, emit_observer_added, emit_observer_removed,
    emit_resolution_submitted,
};
use soroban_sdk::{contract, contractimpl, Address, BytesN, Env, Vec};
use storage::{
    add_case_attested_observer, get_attestation_record, get_case_attested_observers,
    get_case_observer, get_case_record, get_dispute_expiration, get_resolution_record, has_admin,
    has_attestation_record, has_case_record, has_resolution_record, is_observer_registered,
    remove_dispute_expiration, remove_observer_registered, remove_resolution_record, set_admin,
    set_attestation_record, set_case_observer, set_case_record, set_contract_version,
    set_dispute_expiration, set_observer_registered, set_resolution_record,
    DEFAULT_DISPUTE_TTL_LEDGERS, MAX_BATCH_QUERY_SIZE, PROTOCOL_VERSION,
};
use types::{
    Attestation, AttestationRole, BreakCode, CaseStatus, Decision, Observation, ObservationRecord,
    SettlementCase,
};

/// Pure helper: counts valid distinct observer attestations for a case.
/// Authorization is verified at submission time; recorded observer attestations remain valid historically.
fn get_observer_attestation_count(
    env: &Env,
    case_id: &BytesN<32>,
    owner: &Address,
    counterparty: &Option<Address>,
) -> u32 {
    let observers = get_case_attested_observers(env, case_id);
    let mut count: u32 = 0;
    for obs in observers.iter() {
        if &obs == owner {
            continue;
        }
        if let Some(ref cp) = counterparty {
            if &obs == cp {
                continue;
            }
        }
        if let Some(att) = get_attestation_record(env, case_id, &obs) {
            if att.role == AttestationRole::Observer {
                count = count.saturating_add(1);
            }
        }
    }
    count
}

/// Pure helper: checks if observer quorum threshold is satisfied for a case.
fn has_required_observer_quorum(
    env: &Env,
    case_id: &BytesN<32>,
    quorum: u32,
    owner: &Address,
    counterparty: &Option<Address>,
) -> bool {
    get_observer_attestation_count(env, case_id, owner, counterparty) >= quorum
}

/// Pure helper: verifies that distinct observer attestation count meets the required quorum.
fn verify_observer_quorum(
    env: &Env,
    case_id: &BytesN<32>,
    quorum: u32,
    owner: &Address,
    counterparty: &Option<Address>,
) -> Result<(), Error> {
    if !has_required_observer_quorum(env, case_id, quorum, owner, counterparty) {
        return Err(Error::ObserverQuorumNotMet);
    }
    Ok(())
}

#[contract]
pub struct SettlementRegistry;

#[contractimpl]
impl SettlementRegistry {
    /// One-time constructor initializing the contract administrator and protocol version.
    ///
    /// # Indexer Events Emitted:
    /// - `CaseCreated`: emitted on `create_case`
    /// - `ObservationRecorded`: emitted on `record_observation`
    /// - `CaseMatched`: emitted on `record_match`
    /// - `CaseBroken`: emitted on `record_break`
    /// - `AttestationSubmitted`: emitted on `submit_attestation`
    /// - `DisputeOpened`: emitted on `open_dispute`
    /// - `ResolutionSubmitted` & `DisputeResolved`: emitted on `submit_resolution`
    /// - `CaseFinalized`: emitted on `finalize_case`
    /// - `ObserverAdded` & `ObserverRemoved`: emitted on observer registration lifecycle
    ///
    /// # Settlement Lifecycle Transitions Supported:
    /// - Open -> Observed
    /// - Observed -> Matched
    /// - Observed -> Break
    /// - Break -> Disputed
    /// - Disputed -> Resolved
    /// - Matched -> Finalized
    /// - Resolved -> Finalized
    pub fn __constructor(env: Env, admin: Address) -> Result<(), Error> {
        if has_admin(&env) {
            return Err(Error::AlreadyInitialized);
        }
        set_admin(&env, &admin);
        set_contract_version(&env, PROTOCOL_VERSION);
        Ok(())
    }

    /// Admin-only: registers a new authorized settlement observer.
    pub fn add_observer(env: Env, observer: Address) -> Result<(), Error> {
        require_admin_auth(&env)?;
        if is_observer_registered(&env, &observer) {
            return Err(Error::ObserverAlreadyRegistered);
        }
        set_observer_registered(&env, &observer);
        emit_observer_added(&env, &observer);
        Ok(())
    }

    /// Admin-only: revokes an authorized settlement observer.
    pub fn remove_observer(env: Env, observer: Address) -> Result<(), Error> {
        require_admin_auth(&env)?;
        if !is_observer_registered(&env, &observer) {
            return Err(Error::ObserverNotRegistered);
        }
        remove_observer_registered(&env, &observer);
        emit_observer_removed(&env, &observer);
        Ok(())
    }

    /// Owner-authorized: opens a new settlement case with terms commitment and expiry.
    pub fn create_case(
        env: Env,
        case_id: BytesN<32>,
        owner: Address,
        counterparty: Option<Address>,
        terms_commitment: BytesN<32>,
        expires_at_ledger: u32,
    ) -> Result<(), Error> {
        owner.require_auth();

        validate_case_identity(&case_id)?;
        validate_terms_commitment(&terms_commitment)?;

        if has_case_record(&env, &case_id) {
            return Err(Error::CaseAlreadyExists);
        }

        let current_ledger = env.ledger().sequence();
        if expires_at_ledger <= current_ledger {
            return Err(Error::InvalidExpiration);
        }

        if let Some(ref cp) = counterparty {
            if cp == &owner {
                return Err(Error::CounterpartyNotAllowed);
            }
        }

        let case = SettlementCase {
            owner: owner.clone(),
            counterparty: counterparty.clone(),
            terms_commitment,
            expires_at_ledger,
            status: CaseStatus::Open,
            observation: Observation::None,
            decision: Decision::None,
            created_at_ledger: current_ledger,
            finalized_at_ledger: None,
            observer_quorum: storage::DEFAULT_OBSERVER_QUORUM,
            dispute_expires_at_ledger: None,
        };

        set_case_record(&env, &case_id, &case);
        storage::set_case_quorum(&env, &case_id, storage::DEFAULT_OBSERVER_QUORUM);
        emit_case_created(
            &env,
            &case_id,
            &case.owner,
            &case.counterparty,
            case.expires_at_ledger,
        );
        Ok(())
    }

    /// Owner-authorized: configures the required observer quorum threshold for a case.
    pub fn set_case_quorum(env: Env, case_id: BytesN<32>, quorum: u32) -> Result<(), Error> {
        if quorum == 0 {
            return Err(Error::InvalidObserverQuorum);
        }
        if quorum > storage::MAX_OBSERVER_QUORUM {
            return Err(Error::ObserverQuorumExceeded);
        }
        let mut case = get_case_record(&env, &case_id).ok_or(Error::NotFound)?;
        case.owner.require_auth();

        if case.status == CaseStatus::Finalized {
            return Err(Error::InvalidState);
        }

        storage::set_case_quorum(&env, &case_id, quorum);
        case.observer_quorum = quorum;
        set_case_record(&env, &case_id, &case);
        emit_case_quorum_set(&env, &case_id, quorum);
        Ok(())
    }

    /// Reads the configured observer quorum threshold for a case.
    pub fn get_case_quorum(env: Env, case_id: BytesN<32>) -> u32 {
        storage::get_case_quorum(&env, &case_id)
    }

    /// Observer-authorized: records an observed settlement transaction for an open case.
    pub fn record_observation(
        env: Env,
        observer: Address,
        case_id: BytesN<32>,
        tx_hash: BytesN<32>,
        observed_ledger: u32,
        observation_commitment: BytesN<32>,
    ) -> Result<(), Error> {
        require_observer_auth(&env, &observer)?;
        validate_case_identity(&case_id)?;
        validate_observation_evidence(&tx_hash, &observation_commitment)?;

        let mut case = get_case_record(&env, &case_id).ok_or(Error::NotFound)?;
        validate_state_transition(case.status, CaseStatus::Observed)?;

        if case.observation != Observation::None {
            return Err(Error::InvalidState);
        }

        let current_ledger = env.ledger().sequence();
        if observed_ledger == 0 || observed_ledger > current_ledger {
            return Err(Error::InvalidLedger);
        }

        let record = ObservationRecord {
            tx_hash: tx_hash.clone(),
            observation_commitment,
            observed_ledger,
        };

        case.observation = Observation::Observed(record);
        case.status = CaseStatus::Observed;

        set_case_record(&env, &case_id, &case);
        set_case_observer(&env, &case_id, &observer);
        if let Observation::Observed(ref obs) = case.observation {
            emit_observation_recorded(&env, &case_id, &observer, &obs.tx_hash, obs.observed_ledger);
        }
        Ok(())
    }

    /// Observer-authorized: records a matched reconciliation decision.
    pub fn record_match(env: Env, observer: Address, case_id: BytesN<32>) -> Result<(), Error> {
        require_observer_auth(&env, &observer)?;
        validate_case_identity(&case_id)?;

        let mut case = get_case_record(&env, &case_id).ok_or(Error::NotFound)?;
        validate_state_transition(case.status, CaseStatus::Matched)?;

        if case.observation == Observation::None {
            return Err(Error::InvalidState);
        }

        case.decision = Decision::Matched;
        case.status = CaseStatus::Matched;

        set_case_record(&env, &case_id, &case);
        emit_case_matched(&env, &case_id, &observer);
        Ok(())
    }

    /// Observer-authorized: records a reconciliation break decision with standardized break code.
    pub fn record_break(
        env: Env,
        observer: Address,
        case_id: BytesN<32>,
        break_code: BreakCode,
    ) -> Result<(), Error> {
        require_observer_auth(&env, &observer)?;
        validate_case_identity(&case_id)?;

        let mut case = get_case_record(&env, &case_id).ok_or(Error::NotFound)?;
        validate_state_transition(case.status, CaseStatus::Break)?;

        if case.observation == Observation::None {
            return Err(Error::InvalidState);
        }

        case.decision = Decision::Break(break_code);
        case.status = CaseStatus::Break;

        set_case_record(&env, &case_id, &case);
        emit_case_broken(&env, &case_id, &observer, break_code);
        Ok(())
    }

    /// Submits a cryptographic attestation for an active settlement case.
    pub fn submit_attestation(
        env: Env,
        case_id: BytesN<32>,
        role: AttestationRole,
        commitment: BytesN<32>,
    ) -> Result<(), Error> {
        validate_case_identity(&case_id)?;
        validate_attestation_commitment(&commitment)?;

        let case = get_case_record(&env, &case_id).ok_or(Error::NotFound)?;
        if case.status == CaseStatus::Finalized {
            return Err(Error::InvalidState);
        }

        let attestor: Address = match role {
            AttestationRole::Owner => {
                case.owner.require_auth();
                case.owner.clone()
            }
            AttestationRole::Counterparty => {
                let cp = case
                    .counterparty
                    .clone()
                    .ok_or(Error::CounterpartyRequired)?;
                cp.require_auth();
                cp
            }
            AttestationRole::Observer => {
                let obs = get_case_observer(&env, &case_id).ok_or(Error::ObserverNotRegistered)?;
                if obs == case.owner {
                    return Err(Error::Unauthorized);
                }
                if let Some(ref cp) = case.counterparty {
                    if &obs == cp {
                        return Err(Error::Unauthorized);
                    }
                }
                require_observer_auth(&env, &obs)?;
                obs
            }
        };

        if has_attestation_record(&env, &case_id, &attestor) {
            return Err(Error::AttestationAlreadyExists);
        }

        let current_ledger = env.ledger().sequence();
        let attestation = Attestation {
            role,
            commitment,
            attested_at_ledger: current_ledger,
        };

        if role == AttestationRole::Observer {
            add_case_attested_observer(&env, &case_id, &attestor)?;
        }
        set_attestation_record(&env, &case_id, &attestor, &attestation);
        emit_attestation_submitted(&env, &case_id, &attestor, role);
        Ok(())
    }

    /// Observer-authorized: submits an attestation as a registered observer.
    /// Used for multi-observer quorum where multiple distinct observers submit attestations.
    pub fn submit_observer_attestation(
        env: Env,
        case_id: BytesN<32>,
        observer: Address,
        commitment: BytesN<32>,
    ) -> Result<(), Error> {
        validate_case_identity(&case_id)?;
        validate_attestation_commitment(&commitment)?;
        require_observer_auth(&env, &observer)?;
        if !is_observer_registered(&env, &observer) {
            return Err(Error::ObserverNotRegistered);
        }
        let case = get_case_record(&env, &case_id).ok_or(Error::NotFound)?;
        if case.status == CaseStatus::Finalized {
            return Err(Error::InvalidState);
        }
        if observer == case.owner {
            return Err(Error::Unauthorized);
        }
        if let Some(ref cp) = case.counterparty {
            if &observer == cp {
                return Err(Error::Unauthorized);
            }
        }
        if has_attestation_record(&env, &case_id, &observer) {
            return Err(Error::AttestationAlreadyExists);
        }
        add_case_attested_observer(&env, &case_id, &observer)?;
        let current_ledger = env.ledger().sequence();
        let attestation = Attestation {
            role: AttestationRole::Observer,
            commitment,
            attested_at_ledger: current_ledger,
        };
        set_attestation_record(&env, &case_id, &observer, &attestation);
        emit_attestation_submitted(&env, &case_id, &observer, AttestationRole::Observer);
        Ok(())
    }

    /// Reads the list of distinct observer addresses that submitted attestations for a case.
    pub fn get_attested_observers(env: Env, case_id: BytesN<32>) -> soroban_sdk::Vec<Address> {
        storage::get_case_attested_observers(&env, &case_id)
    }

    /// Owner or Counterparty: opens a dispute against a broken settlement case.
    pub fn open_dispute(
        env: Env,
        initiator: Address,
        case_id: BytesN<32>,
        dispute_commitment: BytesN<32>,
    ) -> Result<(), Error> {
        initiator.require_auth();
        validate_case_identity(&case_id)?;
        require_non_zero_commitment(&dispute_commitment)?;

        let mut case = get_case_record(&env, &case_id).ok_or(Error::NotFound)?;

        let is_owner = initiator == case.owner;
        let is_cp = case.counterparty.as_ref() == Some(&initiator);

        if !is_owner && !is_cp {
            return Err(Error::Unauthorized);
        }

        validate_state_transition(case.status, CaseStatus::Disputed)?;

        if case.counterparty.is_none() {
            return Err(Error::CounterpartyRequired);
        }

        let current_ledger = env.ledger().sequence();
        let expiration_ledger = current_ledger.saturating_add(DEFAULT_DISPUTE_TTL_LEDGERS);
        set_dispute_expiration(&env, &case_id, expiration_ledger);
        case.dispute_expires_at_ledger = Some(expiration_ledger);

        case.status = CaseStatus::Disputed;
        set_case_record(&env, &case_id, &case);
        emit_dispute_opened(&env, &case_id, &initiator, &dispute_commitment);
        Ok(())
    }

    /// Owner or Counterparty: opens a dispute against a broken settlement case with custom TTL ledgers.
    pub fn open_dispute_with_ttl(
        env: Env,
        initiator: Address,
        case_id: BytesN<32>,
        dispute_commitment: BytesN<32>,
        ttl_ledgers: u32,
    ) -> Result<(), Error> {
        initiator.require_auth();
        validate_case_identity(&case_id)?;
        require_non_zero_commitment(&dispute_commitment)?;

        let mut case = get_case_record(&env, &case_id).ok_or(Error::NotFound)?;

        let is_owner = initiator == case.owner;
        let is_cp = case.counterparty.as_ref() == Some(&initiator);

        if !is_owner && !is_cp {
            return Err(Error::Unauthorized);
        }

        validate_state_transition(case.status, CaseStatus::Disputed)?;

        if case.counterparty.is_none() {
            return Err(Error::CounterpartyRequired);
        }

        if !(storage::MIN_DISPUTE_TTL_LEDGERS..=storage::MAX_DISPUTE_TTL_LEDGERS)
            .contains(&ttl_ledgers)
        {
            return Err(Error::InvalidExpiration);
        }

        let current_ledger = env.ledger().sequence();
        let expiration_ledger = current_ledger.saturating_add(ttl_ledgers);
        set_dispute_expiration(&env, &case_id, expiration_ledger);
        case.dispute_expires_at_ledger = Some(expiration_ledger);

        case.status = CaseStatus::Disputed;
        set_case_record(&env, &case_id, &case);
        emit_dispute_opened(&env, &case_id, &initiator, &dispute_commitment);
        Ok(())
    }

    /// Permissionless: triggers expiration of an unaddressed dispute after TTL expires.
    /// Transitions case back to Break with auto-break resolution.
    pub fn expire_dispute(env: Env, case_id: BytesN<32>) -> Result<(), Error> {
        validate_case_identity(&case_id)?;
        let mut case = get_case_record(&env, &case_id).ok_or(Error::NotFound)?;

        if case.status != CaseStatus::Disputed {
            return Err(Error::InvalidState);
        }

        let exp = get_dispute_expiration(&env, &case_id).ok_or(Error::DisputeNotExpired)?;
        let current_ledger = env.ledger().sequence();
        if current_ledger < exp {
            return Err(Error::DisputeNotExpired);
        }

        validate_state_transition(case.status, CaseStatus::Break)?;

        remove_dispute_expiration(&env, &case_id);
        remove_resolution_record(&env, &case_id, &case.owner);
        if let Some(ref cp) = case.counterparty {
            remove_resolution_record(&env, &case_id, cp);
        }
        case.dispute_expires_at_ledger = None;
        case.status = CaseStatus::Break;
        set_case_record(&env, &case_id, &case);
        emit_dispute_expired(&env, &case_id, exp, current_ledger);
        Ok(())
    }

    /// Reads the dispute expiration ledger sequence for an active dispute, if any.
    pub fn get_dispute_expiration(env: Env, case_id: BytesN<32>) -> Option<u32> {
        storage::get_dispute_expiration(&env, &case_id)
    }

    /// Owner or Counterparty: submits two-party resolution commitment.
    pub fn submit_resolution(
        env: Env,
        resolver: Address,
        case_id: BytesN<32>,
        resolution_commitment: BytesN<32>,
    ) -> Result<(), Error> {
        resolver.require_auth();
        validate_case_identity(&case_id)?;
        require_non_zero_commitment(&resolution_commitment)?;

        let mut case = get_case_record(&env, &case_id).ok_or(Error::NotFound)?;
        if case.status != CaseStatus::Disputed {
            return Err(Error::InvalidState);
        }

        if let Some(exp) = get_dispute_expiration(&env, &case_id) {
            if env.ledger().sequence() >= exp {
                return Err(Error::DisputeAlreadyExpired);
            }
        }

        let is_owner = resolver == case.owner;
        let is_cp = case.counterparty.as_ref() == Some(&resolver);

        if !is_owner && !is_cp {
            return Err(Error::Unauthorized);
        }

        if has_resolution_record(&env, &case_id, &resolver) {
            return Err(Error::ResolutionAlreadySubmitted);
        }

        set_resolution_record(&env, &case_id, &resolver, &resolution_commitment);
        emit_resolution_submitted(&env, &case_id, &resolver, &resolution_commitment);

        // Check if both parties have submitted and commitments match
        let owner_res = get_resolution_record(&env, &case_id, &case.owner);
        let cp_res = if let Some(ref cp) = case.counterparty {
            get_resolution_record(&env, &case_id, cp)
        } else {
            None
        };

        if let (Some(o_comm), Some(c_comm)) = (owner_res, cp_res) {
            if o_comm == c_comm {
                validate_state_transition(case.status, CaseStatus::Resolved)?;
                case.status = CaseStatus::Resolved;
                remove_dispute_expiration(&env, &case_id);
                case.dispute_expires_at_ledger = None;
                set_case_record(&env, &case_id, &case);
                emit_dispute_resolved(&env, &case_id, &o_comm);
            }
        }

        Ok(())
    }

    /// Owner-authorized: finalizes a matched or resolved settlement case.
    pub fn finalize_case(env: Env, case_id: BytesN<32>) -> Result<(), Error> {
        validate_case_identity(&case_id)?;
        let mut case = get_case_record(&env, &case_id).ok_or(Error::NotFound)?;
        case.owner.require_auth();

        match case.status {
            CaseStatus::Matched => {
                validate_state_transition(case.status, CaseStatus::Finalized)?;

                // Require owner attestation
                let owner_att = get_attestation_record(&env, &case_id, &case.owner)
                    .ok_or(Error::MissingRequiredAttestation)?;
                if owner_att.role != AttestationRole::Owner {
                    return Err(Error::MissingRequiredAttestation);
                }

                // Verify pure observer quorum threshold
                verify_observer_quorum(
                    &env,
                    &case_id,
                    case.observer_quorum,
                    &case.owner,
                    &case.counterparty,
                )?;
            }
            CaseStatus::Resolved => {
                validate_state_transition(case.status, CaseStatus::Finalized)?;

                // Require owner attestation
                let owner_att = get_attestation_record(&env, &case_id, &case.owner)
                    .ok_or(Error::MissingRequiredAttestation)?;
                if owner_att.role != AttestationRole::Owner {
                    return Err(Error::MissingRequiredAttestation);
                }

                // Require counterparty attestation
                let cp = case
                    .counterparty
                    .clone()
                    .ok_or(Error::CounterpartyRequired)?;
                let cp_att = get_attestation_record(&env, &case_id, &cp)
                    .ok_or(Error::MissingRequiredAttestation)?;
                if cp_att.role != AttestationRole::Counterparty {
                    return Err(Error::MissingRequiredAttestation);
                }

                // Verify pure observer quorum threshold
                verify_observer_quorum(
                    &env,
                    &case_id,
                    case.observer_quorum,
                    &case.owner,
                    &case.counterparty,
                )?;
            }
            _ => {
                return Err(Error::InvalidState);
            }
        }

        let current_ledger = env.ledger().sequence();
        case.status = CaseStatus::Finalized;
        case.finalized_at_ledger = Some(current_ledger);

        set_case_record(&env, &case_id, &case);
        emit_case_finalized(&env, &case_id, current_ledger);
        Ok(())
    }

    /// Reads a settlement case record by ID.
    pub fn get_case(env: Env, case_id: BytesN<32>) -> Result<SettlementCase, Error> {
        let mut case = get_case_record(&env, &case_id).ok_or(Error::NotFound)?;
        case.observer_quorum = storage::get_case_quorum(&env, &case_id);
        case.dispute_expires_at_ledger = storage::get_dispute_expiration(&env, &case_id);
        Ok(case)
    }

    /// Bulk-reads settlement case records by a list of case IDs, returning None for cases that do not exist.
    /// Bounded by MAX_BATCH_QUERY_SIZE (50) to prevent resource exhaustion.
    pub fn get_cases_batch(
        env: Env,
        case_ids: Vec<BytesN<32>>,
    ) -> Result<Vec<Option<SettlementCase>>, Error> {
        if case_ids.len() > MAX_BATCH_QUERY_SIZE {
            return Err(Error::BatchLimitExceeded);
        }

        let mut results = Vec::new(&env);
        for case_id in case_ids.iter() {
            let record = match get_case_record(&env, &case_id) {
                Some(mut case) => {
                    case.observer_quorum = storage::get_case_quorum(&env, &case_id);
                    case.dispute_expires_at_ledger =
                        storage::get_dispute_expiration(&env, &case_id);
                    Some(case)
                }
                None => None,
            };
            results.push_back(record);
        }
        Ok(results)
    }

    /// Reads an attestation record by case ID and attestor address.
    pub fn get_attestation(
        env: Env,
        case_id: BytesN<32>,
        attestor: Address,
    ) -> Option<Attestation> {
        get_attestation_record(&env, &case_id, &attestor)
    }

    /// Checks whether an address is a registered observer.
    pub fn is_observer(env: Env, observer: Address) -> bool {
        is_observer_registered(&env, &observer)
    }

    /// Reads a resolution commitment by case ID and resolver address.
    pub fn get_resolution(env: Env, case_id: BytesN<32>, resolver: Address) -> Option<BytesN<32>> {
        get_resolution_record(&env, &case_id, &resolver)
    }
}
