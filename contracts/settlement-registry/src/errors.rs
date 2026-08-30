use soroban_sdk::contracterror;

/// Typed contract errors with stable numeric discriminants for StellarClear SettlementRegistry.
#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// Contract is already initialized.
    AlreadyInitialized = 1,
    /// Requested record was not found.
    NotFound = 2,
    /// Case with the given identifier already exists.
    CaseAlreadyExists = 3,
    /// Observer address is already registered in the registry.
    ObserverAlreadyRegistered = 4,
    /// Observer address is not registered in the registry.
    ObserverNotRegistered = 5,
    /// Caller is not authorized to perform the operation.
    Unauthorized = 6,
    /// Target case is not in a valid state for the requested operation.
    InvalidState = 7,
    /// Expiration ledger must be strictly greater than the current ledger.
    InvalidExpiration = 8,
    /// Commitment is invalid or all zeros.
    InvalidCommitment = 9,
    /// Operation requires a counterparty, but none was defined on the case.
    CounterpartyRequired = 10,
    /// Counterparty cannot be the same as the case owner.
    CounterpartyNotAllowed = 11,
    /// Attestation already submitted by this address for the specified case.
    AttestationAlreadyExists = 12,
    /// Resolution commitment has already been submitted by this party.
    ResolutionAlreadySubmitted = 13,
    /// Resolution commitments between owner and counterparty do not match.
    ResolutionMismatch = 14,
    /// Required attestation is missing to finalize the case.
    MissingRequiredAttestation = 15,
    /// Decision is invalid for the case state.
    InvalidDecision = 16,
    /// Observation ledger is zero or in the future relative to current ledger.
    InvalidLedger = 17,
}
