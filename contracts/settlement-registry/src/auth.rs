use crate::errors::Error;
use crate::storage::{get_admin, is_observer_registered};
use crate::types::CaseStatus;
use soroban_sdk::{Address, BytesN, Env};

/// Verifies that the administrator called and authenticated this transaction.
pub fn require_admin_auth(env: &Env) -> Result<Address, Error> {
    let admin = get_admin(env).ok_or(Error::Unauthorized)?;
    admin.require_auth();
    Ok(admin)
}

/// Verifies that the given observer is registered and authenticated.
pub fn require_observer_auth(env: &Env, observer: &Address) -> Result<(), Error> {
    observer.require_auth();
    if !is_observer_registered(env, observer) {
        return Err(Error::ObserverNotRegistered);
    }
    Ok(())
}

/// Validates that a 32-byte cryptographic commitment is non-zero.
pub fn require_non_zero_commitment(commitment: &BytesN<32>) -> Result<(), Error> {
    let bytes = commitment.to_array();
    if bytes.iter().all(|&b| b == 0) {
        return Err(Error::InvalidCommitment);
    }
    Ok(())
}

/// Centralized validator for legal settlement case state transitions.
pub fn validate_state_transition(current: CaseStatus, next: CaseStatus) -> Result<(), Error> {
    let legal = matches!(
        (current, next),
        (CaseStatus::Open, CaseStatus::Observed)
            | (CaseStatus::Observed, CaseStatus::Matched)
            | (CaseStatus::Observed, CaseStatus::Break)
            | (CaseStatus::Break, CaseStatus::Disputed)
            | (CaseStatus::Disputed, CaseStatus::Resolved)
            | (CaseStatus::Matched, CaseStatus::Finalized)
            | (CaseStatus::Resolved, CaseStatus::Finalized)
    );

    if legal {
        Ok(())
    } else {
        Err(Error::InvalidState)
    }
}
