use soroban_sdk::{contracttype, Address, BytesN};

/// Lifecycle state for a settlement case.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CaseStatus {
    Open,
    Observed,
    Matched,
    Break,
    Disputed,
    Resolved,
    Finalized,
}

/// Standardized classification for reconciliation breaks.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BreakCode {
    AmountMismatch,
    AssetMismatch,
    DestinationMismatch,
    ReferenceMismatch,
    MissingSettlement,
    DuplicateSettlement,
    LateSettlement,
    FailedTransaction,
    UnexpectedTransaction,
}

/// Reconciliation decision recorded on-chain by a registered observer.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Decision {
    None,
    Matched,
    Break(BreakCode),
}

/// Stellar settlement observation details recorded by an observer.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservationRecord {
    /// Stellar transaction hash where settlement was observed.
    pub tx_hash: BytesN<32>,
    /// Opaque 32-byte cryptographic commitment to observation details.
    pub observation_commitment: BytesN<32>,
    /// Stellar ledger sequence where the transaction occurred.
    pub observed_ledger: u32,
}

/// Settlement observation state for a case.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Observation {
    None,
    Observed(ObservationRecord),
}

/// Participant role for an attestation.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttestationRole {
    Owner,
    Counterparty,
    Observer,
}

/// Cryptographic attestation anchoring an authorized party's confirmation.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Attestation {
    /// Role under which this attestation was submitted.
    pub role: AttestationRole,
    /// 32-byte commitment payload.
    pub commitment: BytesN<32>,
    /// Stellar ledger sequence when attestation was recorded.
    pub attested_at_ledger: u32,
}

/// Core protocol object anchoring settlement terms, observations, and decisions.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettlementCase {
    /// Initiating owner / originator of the settlement case.
    pub owner: Address,
    /// Optional counterparty address expected to participate or settle.
    pub counterparty: Option<Address>,
    /// 32-byte hash commitment of expected settlement terms.
    pub terms_commitment: BytesN<32>,
    /// Expiration ledger sequence for settlement.
    pub expires_at_ledger: u32,
    /// Current lifecycle state.
    pub status: CaseStatus,
    /// Settlement observation recorded by an authorized observer.
    pub observation: Observation,
    /// Reconciliation decision.
    pub decision: Decision,
    /// Ledger sequence when the case was created.
    pub created_at_ledger: u32,
    /// Ledger sequence when the case reached finalization.
    pub finalized_at_ledger: Option<u32>,
}
