use crate::types::{AttestationRole, BreakCode};
use soroban_sdk::{contractevent, Address, BytesN, Env};

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObserverAdded {
    #[topic]
    pub observer: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObserverRemoved {
    #[topic]
    pub observer: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaseCreated {
    #[topic]
    pub case_id: BytesN<32>,
    pub owner: Address,
    pub counterparty: Option<Address>,
    pub expires_at_ledger: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservationRecorded {
    #[topic]
    pub case_id: BytesN<32>,
    pub observer: Address,
    pub tx_hash: BytesN<32>,
    pub observed_ledger: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaseMatched {
    #[topic]
    pub case_id: BytesN<32>,
    pub observer: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaseBroken {
    #[topic]
    pub case_id: BytesN<32>,
    pub observer: Address,
    pub break_code: BreakCode,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttestationSubmitted {
    #[topic]
    pub case_id: BytesN<32>,
    pub attestor: Address,
    pub role: AttestationRole,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeOpened {
    #[topic]
    pub case_id: BytesN<32>,
    pub initiator: Address,
    pub dispute_commitment: BytesN<32>,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolutionSubmitted {
    #[topic]
    pub case_id: BytesN<32>,
    pub resolver: Address,
    pub resolution_commitment: BytesN<32>,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeResolved {
    #[topic]
    pub case_id: BytesN<32>,
    pub resolution_commitment: BytesN<32>,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaseFinalized {
    #[topic]
    pub case_id: BytesN<32>,
    pub finalized_at_ledger: u32,
}

pub fn emit_observer_added(env: &Env, observer: &Address) {
    ObserverAdded {
        observer: observer.clone(),
    }
    .publish(env);
}

pub fn emit_observer_removed(env: &Env, observer: &Address) {
    ObserverRemoved {
        observer: observer.clone(),
    }
    .publish(env);
}

pub fn emit_case_created(
    env: &Env,
    case_id: &BytesN<32>,
    owner: &Address,
    counterparty: &Option<Address>,
    expires_at_ledger: u32,
) {
    CaseCreated {
        case_id: case_id.clone(),
        owner: owner.clone(),
        counterparty: counterparty.clone(),
        expires_at_ledger,
    }
    .publish(env);
}

pub fn emit_observation_recorded(
    env: &Env,
    case_id: &BytesN<32>,
    observer: &Address,
    tx_hash: &BytesN<32>,
    observed_ledger: u32,
) {
    ObservationRecorded {
        case_id: case_id.clone(),
        observer: observer.clone(),
        tx_hash: tx_hash.clone(),
        observed_ledger,
    }
    .publish(env);
}

pub fn emit_case_matched(env: &Env, case_id: &BytesN<32>, observer: &Address) {
    CaseMatched {
        case_id: case_id.clone(),
        observer: observer.clone(),
    }
    .publish(env);
}

pub fn emit_case_broken(
    env: &Env,
    case_id: &BytesN<32>,
    observer: &Address,
    break_code: BreakCode,
) {
    CaseBroken {
        case_id: case_id.clone(),
        observer: observer.clone(),
        break_code,
    }
    .publish(env);
}

pub fn emit_attestation_submitted(
    env: &Env,
    case_id: &BytesN<32>,
    attestor: &Address,
    role: AttestationRole,
) {
    AttestationSubmitted {
        case_id: case_id.clone(),
        attestor: attestor.clone(),
        role,
    }
    .publish(env);
}

pub fn emit_dispute_opened(
    env: &Env,
    case_id: &BytesN<32>,
    initiator: &Address,
    dispute_commitment: &BytesN<32>,
) {
    DisputeOpened {
        case_id: case_id.clone(),
        initiator: initiator.clone(),
        dispute_commitment: dispute_commitment.clone(),
    }
    .publish(env);
}

pub fn emit_resolution_submitted(
    env: &Env,
    case_id: &BytesN<32>,
    resolver: &Address,
    resolution_commitment: &BytesN<32>,
) {
    ResolutionSubmitted {
        case_id: case_id.clone(),
        resolver: resolver.clone(),
        resolution_commitment: resolution_commitment.clone(),
    }
    .publish(env);
}

pub fn emit_dispute_resolved(env: &Env, case_id: &BytesN<32>, resolution_commitment: &BytesN<32>) {
    DisputeResolved {
        case_id: case_id.clone(),
        resolution_commitment: resolution_commitment.clone(),
    }
    .publish(env);
}

pub fn emit_case_finalized(env: &Env, case_id: &BytesN<32>, finalized_at_ledger: u32) {
    CaseFinalized {
        case_id: case_id.clone(),
        finalized_at_ledger,
    }
    .publish(env);
}
