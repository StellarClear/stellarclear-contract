use crate::types::{Attestation, SettlementCase};
use soroban_sdk::{contracttype, Address, BytesN, Env};

/// Storage key definitions for instance and persistent contract storage.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Administrator address (instance storage).
    Admin,
    /// Protocol version constant (instance storage).
    ContractVersion,
    /// Registered observer address flag (persistent storage).
    Observer(Address),
    /// Settlement case record indexed by 32-byte case ID (persistent storage).
    Case(BytesN<32>),
    /// Registered observer who recorded the observation for a case (persistent storage).
    CaseObserver(BytesN<32>),
    /// Attestation indexed by case ID and attestor address (persistent storage).
    Attestation(BytesN<32>, Address),
    /// Resolution commitment indexed by case ID and resolver address (persistent storage).
    Resolution(BytesN<32>, Address),
    /// Configured observer quorum threshold for a case (persistent storage).
    CaseQuorum(BytesN<32>),
    /// List of distinct observer addresses that submitted attestations for a case (persistent storage).
    CaseAttestedObservers(BytesN<32>),
    /// Dispute expiration ledger sequence indexed by case ID (persistent storage).
    DisputeExpiration(BytesN<32>),
}

// Protocol version constant
pub const PROTOCOL_VERSION: u32 = 1;

/// Default observer quorum threshold for newly created settlement cases.
pub const DEFAULT_OBSERVER_QUORUM: u32 = 1;

/// Maximum allowed observer quorum threshold per case to guarantee bounded execution.
pub const MAX_OBSERVER_QUORUM: u32 = 10;

/// Maximum distinct authorized observers that can attest per case to bound storage size.
pub const MAX_OBSERVERS_PER_CASE: u32 = 16;

/// Default dispute expiration TTL in ledgers (~24 hours at 5s/ledger).
pub const DEFAULT_DISPUTE_TTL_LEDGERS: u32 = 17_280;

/// Minimum dispute expiration TTL in ledgers (~10 minutes at 5s/ledger).
pub const MIN_DISPUTE_TTL_LEDGERS: u32 = 120;

/// Maximum dispute expiration TTL in ledgers (~30 days at 5s/ledger).
pub const MAX_DISPUTE_TTL_LEDGERS: u32 = 518_400;

/// Maximum number of cases that can be queried in a single batch call.
pub const MAX_BATCH_QUERY_SIZE: u32 = 50;

// Storage helpers for Admin
pub fn get_admin(env: &Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::Admin)
}

pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&DataKey::Admin, admin);
}

pub fn has_admin(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::Admin)
}

// Storage helpers for Contract Version
pub fn get_contract_version(env: &Env) -> Option<u32> {
    env.storage().instance().get(&DataKey::ContractVersion)
}

pub fn set_contract_version(env: &Env, version: u32) {
    env.storage()
        .instance()
        .set(&DataKey::ContractVersion, &version);
}

// Storage helpers for Observers (persistent)
pub fn is_observer_registered(env: &Env, observer: &Address) -> bool {
    let key = DataKey::Observer(observer.clone());
    env.storage().persistent().has(&key)
}

pub fn set_observer_registered(env: &Env, observer: &Address) {
    let key = DataKey::Observer(observer.clone());
    env.storage().persistent().set(&key, &true);
}

pub fn remove_observer_registered(env: &Env, observer: &Address) {
    let key = DataKey::Observer(observer.clone());
    env.storage().persistent().remove(&key);
}

// Storage helpers for SettlementCase (persistent)
pub fn get_case_record(env: &Env, case_id: &BytesN<32>) -> Option<SettlementCase> {
    let key = DataKey::Case(case_id.clone());
    env.storage().persistent().get(&key)
}

pub fn set_case_record(env: &Env, case_id: &BytesN<32>, case: &SettlementCase) {
    let key = DataKey::Case(case_id.clone());
    env.storage().persistent().set(&key, case);
}

pub fn has_case_record(env: &Env, case_id: &BytesN<32>) -> bool {
    let key = DataKey::Case(case_id.clone());
    env.storage().persistent().has(&key)
}

// Storage helpers for CaseObserver (persistent)
pub fn get_case_observer(env: &Env, case_id: &BytesN<32>) -> Option<Address> {
    let key = DataKey::CaseObserver(case_id.clone());
    env.storage().persistent().get(&key)
}

pub fn set_case_observer(env: &Env, case_id: &BytesN<32>, observer: &Address) {
    let key = DataKey::CaseObserver(case_id.clone());
    env.storage().persistent().set(&key, observer);
}

// Storage helpers for Attestations (persistent)
pub fn get_attestation_record(
    env: &Env,
    case_id: &BytesN<32>,
    attestor: &Address,
) -> Option<Attestation> {
    let key = DataKey::Attestation(case_id.clone(), attestor.clone());
    env.storage().persistent().get(&key)
}

pub fn set_attestation_record(
    env: &Env,
    case_id: &BytesN<32>,
    attestor: &Address,
    attestation: &Attestation,
) {
    let key = DataKey::Attestation(case_id.clone(), attestor.clone());
    env.storage().persistent().set(&key, attestation);
}

pub fn has_attestation_record(env: &Env, case_id: &BytesN<32>, attestor: &Address) -> bool {
    let key = DataKey::Attestation(case_id.clone(), attestor.clone());
    env.storage().persistent().has(&key)
}

// Storage helpers for Resolutions (persistent)
pub fn get_resolution_record(
    env: &Env,
    case_id: &BytesN<32>,
    resolver: &Address,
) -> Option<BytesN<32>> {
    let key = DataKey::Resolution(case_id.clone(), resolver.clone());
    env.storage().persistent().get(&key)
}

pub fn set_resolution_record(
    env: &Env,
    case_id: &BytesN<32>,
    resolver: &Address,
    commitment: &BytesN<32>,
) {
    let key = DataKey::Resolution(case_id.clone(), resolver.clone());
    env.storage().persistent().set(&key, commitment);
}

pub fn has_resolution_record(env: &Env, case_id: &BytesN<32>, resolver: &Address) -> bool {
    let key = DataKey::Resolution(case_id.clone(), resolver.clone());
    env.storage().persistent().has(&key)
}

pub fn remove_resolution_record(env: &Env, case_id: &BytesN<32>, resolver: &Address) {
    let key = DataKey::Resolution(case_id.clone(), resolver.clone());
    env.storage().persistent().remove(&key);
}

// Storage helpers for Case Quorum (persistent)
pub fn get_case_quorum(env: &Env, case_id: &BytesN<32>) -> u32 {
    let key = DataKey::CaseQuorum(case_id.clone());
    env.storage()
        .persistent()
        .get(&key)
        .unwrap_or(DEFAULT_OBSERVER_QUORUM)
}

pub fn set_case_quorum(env: &Env, case_id: &BytesN<32>, quorum: u32) {
    let key = DataKey::CaseQuorum(case_id.clone());
    env.storage().persistent().set(&key, &quorum);
}

// Storage helpers for Case Attested Observers (persistent)
pub fn get_case_attested_observers(env: &Env, case_id: &BytesN<32>) -> soroban_sdk::Vec<Address> {
    let key = DataKey::CaseAttestedObservers(case_id.clone());
    env.storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| soroban_sdk::Vec::new(env))
}

pub fn set_case_attested_observers(
    env: &Env,
    case_id: &BytesN<32>,
    observers: &soroban_sdk::Vec<Address>,
) {
    let key = DataKey::CaseAttestedObservers(case_id.clone());
    env.storage().persistent().set(&key, observers);
}

pub fn add_case_attested_observer(
    env: &Env,
    case_id: &BytesN<32>,
    observer: &Address,
) -> Result<(), crate::errors::Error> {
    let mut observers = get_case_attested_observers(env, case_id);
    let mut already_present = false;
    for existing in observers.iter() {
        if &existing == observer {
            already_present = true;
            break;
        }
    }
    if !already_present {
        if observers.len() >= MAX_OBSERVERS_PER_CASE {
            return Err(crate::errors::Error::ObserverLimitExceeded);
        }
        observers.push_back(observer.clone());
        set_case_attested_observers(env, case_id, &observers);
    }
    Ok(())
}

// Storage helpers for Dispute Expiration (persistent)
pub fn get_dispute_expiration(env: &Env, case_id: &BytesN<32>) -> Option<u32> {
    let key = DataKey::DisputeExpiration(case_id.clone());
    env.storage().persistent().get(&key)
}

pub fn set_dispute_expiration(env: &Env, case_id: &BytesN<32>, expiration_ledger: u32) {
    let key = DataKey::DisputeExpiration(case_id.clone());
    env.storage().persistent().set(&key, &expiration_ledger);
}

pub fn remove_dispute_expiration(env: &Env, case_id: &BytesN<32>) {
    let key = DataKey::DisputeExpiration(case_id.clone());
    env.storage().persistent().remove(&key);
}
