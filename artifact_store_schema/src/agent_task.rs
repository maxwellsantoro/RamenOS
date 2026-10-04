//! SW0 task schemas, pure A0 reference model and private host-worker records.
//! No IO, credentials or enforcement backend live in these serializable types.
//!
//! Authority, monotonic time, content hashes, and validator outcomes are supplied
//! by a trusted executor. These serializable records are not credentials or
//! authenticated attestations. A1 must bind them to real service operations,
//! durable receipts, and a supervised validator before making boundary claims.

use crate::ContentId;
use crate::prelude::*;
use serde::{Deserialize, Serialize};

const MAX_RECORDS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractError {
    InvalidContract,
    InvalidContentId,
    WrongAuthority,
    WrongKind,
    Revoked,
    Expired,
    NotStaged,
    ValidationMismatch,
    InvalidCandidate,
    ExecutionLimit,
    Conflict,
    RequestReuse,
    Capacity,
    RevisionExhausted,
}

impl core::fmt::Display for ContractError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "agent task contract: {self:?}")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ContractError {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExecutionBudgetV0 {
    pub guest_ms: u64,
    pub wall_ms: u64,
    pub host_call_ms: u64,
    pub max_diagnostics_bytes: u64,
}

impl ExecutionBudgetV0 {
    pub fn validate(&self) -> Result<(), ContractError> {
        if self.guest_ms == 0
            || self.wall_ms == 0
            || self.wall_ms > 300_000
            || self.host_call_ms == 0
            || self.guest_ms > self.wall_ms
            || self.host_call_ms > self.wall_ms
            || self.max_diagnostics_bytes == 0
            || self.max_diagnostics_bytes > 65_536
        {
            return Err(ContractError::InvalidContract);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TaskContractV0 {
    pub schema_version: u32,
    pub task_id: u64,
    pub domain_id: u64,
    /// Exact logical output identity, never a host filesystem path.
    pub resource: String,
    pub schema_id: String,
    pub policy_id: String,
    pub validator_id: String,
    pub execution_budget: ExecutionBudgetV0,
}

impl TaskContractV0 {
    pub fn validate(&self) -> Result<(), ContractError> {
        if self.schema_version != 1
            || self.task_id == 0
            || self.domain_id == 0
            || self.resource.is_empty()
            || self.resource.len() > 128
            || !self
                .resource
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b':' | b'/' | b'_' | b'-' | b'.'))
        {
            return Err(ContractError::InvalidContract);
        }
        for id in [&self.schema_id, &self.policy_id, &self.validator_id] {
            valid_id(id)?;
        }
        self.execution_budget.validate()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GrantKindV0 {
    Read,
    Mutation,
}

/// A0 authority input, not a token an agent can mint and submit to a service.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TaskGrantV0 {
    pub task_id: u64,
    pub domain_id: u64,
    pub resource: String,
    pub generation: u64,
    pub expires_at_ms: u64,
    pub kind: GrantKindV0,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValidationOutcomeV0 {
    Valid,
    Invalid,
    Timeout,
    HostFailure,
}

/// Trusted executor observation. Serialization alone provides no authenticity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ValidationEvidenceV0 {
    pub task_id: u64,
    pub domain_id: u64,
    pub resource: String,
    pub grant_generation: u64,
    pub candidate_id: String,
    pub schema_id: String,
    pub policy_id: String,
    pub validator_id: String,
    pub valid_until_ms: u64,
    pub wall_elapsed_ms: u64,
    pub guest_elapsed_ms: u64,
    pub diagnostics_bytes: u64,
    pub diagnostics_truncated: bool,
    pub outcome: ValidationOutcomeV0,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CommitRequestV0 {
    pub request_id: u64,
    pub task_id: u64,
    pub domain_id: u64,
    pub resource: String,
    pub candidate_id: String,
    pub expected_revision: u64,
    pub expected_content_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OutputRevisionV0 {
    pub revision: u64,
    pub content_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CommitReceiptV0 {
    pub request_id: u64,
    pub task_id: u64,
    pub domain_id: u64,
    pub resource: String,
    pub grant_generation: u64,
    pub prior: OutputRevisionV0,
    pub accepted: OutputRevisionV0,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StagedCandidate {
    content_id: String,
    validation: Option<ValidationEvidenceV0>,
}

/// In-memory reference semantics only. A1 owns durable atomic publication.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskStateV0 {
    contract: TaskContractV0,
    accepted: OutputRevisionV0,
    generation: u64,
    staged: Vec<StagedCandidate>,
    receipts: Vec<(CommitRequestV0, CommitReceiptV0)>,
}

fn valid_id(id: &str) -> Result<(), ContractError> {
    ContentId::parse(id)
        .map(|_| ())
        .map_err(|_| ContractError::InvalidContentId)
}

impl TaskStateV0 {
    /// Validate a trusted service journal after deserialization. This is not
    /// authentication and must never accept an agent-supplied snapshot.
    pub fn validate_journal(&self) -> Result<(), ContractError> {
        self.contract.validate()?;
        valid_id(&self.accepted.content_id)?;
        if self.generation == 0
            || self.staged.len() > MAX_RECORDS
            || self.receipts.len() > MAX_RECORDS
        {
            return Err(ContractError::InvalidContract);
        }
        for (index, candidate) in self.staged.iter().enumerate() {
            valid_id(&candidate.content_id)?;
            if self.staged[..index]
                .iter()
                .any(|c| c.content_id == candidate.content_id)
            {
                return Err(ContractError::InvalidContract);
            }
            if let Some(e) = &candidate.validation {
                if e.candidate_id != candidate.content_id
                    || e.task_id != self.contract.task_id
                    || e.domain_id != self.contract.domain_id
                    || e.resource != self.contract.resource
                    || e.schema_id != self.contract.schema_id
                    || e.policy_id != self.contract.policy_id
                    || e.validator_id != self.contract.validator_id
                    || e.grant_generation == 0
                    || e.grant_generation > self.generation
                {
                    return Err(ContractError::ValidationMismatch);
                }
            }
        }
        for (index, (request, receipt)) in self.receipts.iter().enumerate() {
            valid_id(&request.candidate_id)?;
            valid_id(&request.expected_content_id)?;
            if request.request_id == 0
                || request.task_id != self.contract.task_id
                || request.domain_id != self.contract.domain_id
                || request.resource != self.contract.resource
                || receipt.request_id != request.request_id
                || receipt.task_id != request.task_id
                || receipt.domain_id != request.domain_id
                || receipt.resource != request.resource
                || receipt.grant_generation == 0
                || receipt.grant_generation > self.generation
                || receipt.prior.revision != request.expected_revision
                || receipt.prior.content_id != request.expected_content_id
                || receipt.accepted.content_id != request.candidate_id
                || receipt.accepted.revision != index as u64 + 1
                || receipt.prior.revision != index as u64
                || self.receipts[..index]
                    .iter()
                    .any(|(r, _)| r.request_id == request.request_id)
                || (index > 0 && receipt.prior != self.receipts[index - 1].1.accepted)
            {
                return Err(ContractError::InvalidContract);
            }
        }
        if self.accepted.revision != self.receipts.len() as u64
            || self
                .receipts
                .last()
                .is_some_and(|(_, r)| r.accepted != self.accepted)
        {
            return Err(ContractError::InvalidContract);
        }
        Ok(())
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn committed_receipts(&self) -> impl Iterator<Item = (&CommitRequestV0, &CommitReceiptV0)> {
        self.receipts
            .iter()
            .map(|(request, receipt)| (request, receipt))
    }

    pub fn candidate_validation(&self, content_id: &str) -> Option<&ValidationEvidenceV0> {
        self.staged
            .iter()
            .find(|entry| entry.content_id == content_id)?
            .validation
            .as_ref()
    }

    pub fn new(contract: TaskContractV0, initial_content_id: &str) -> Result<Self, ContractError> {
        contract.validate()?;
        valid_id(initial_content_id)?;
        Ok(Self {
            contract,
            accepted: OutputRevisionV0 {
                revision: 0,
                content_id: initial_content_id.into(),
            },
            generation: 1,
            staged: Vec::new(),
            receipts: Vec::new(),
        })
    }

    pub fn contract(&self) -> &TaskContractV0 {
        &self.contract
    }

    pub fn accepted(&self) -> &OutputRevisionV0 {
        &self.accepted
    }

    fn authorize(&self, grant: &TaskGrantV0, now_ms: u64) -> Result<(), ContractError> {
        if grant.task_id != self.contract.task_id
            || grant.domain_id != self.contract.domain_id
            || grant.resource != self.contract.resource
        {
            return Err(ContractError::WrongAuthority);
        }
        if grant.kind != GrantKindV0::Mutation {
            return Err(ContractError::WrongKind);
        }
        if grant.generation != self.generation {
            return Err(ContractError::Revoked);
        }
        if now_ms >= grant.expires_at_ms {
            return Err(ContractError::Expired);
        }
        Ok(())
    }

    pub fn revoke(&mut self) -> Result<(), ContractError> {
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or(ContractError::RevisionExhausted)?;
        Ok(())
    }

    pub fn stage(
        &mut self,
        grant: &TaskGrantV0,
        candidate_id: &str,
        now_ms: u64,
    ) -> Result<(), ContractError> {
        self.authorize(grant, now_ms)?;
        valid_id(candidate_id)?;
        if self
            .staged
            .iter()
            .any(|entry| entry.content_id == candidate_id)
        {
            return Ok(());
        }
        if self.staged.len() == MAX_RECORDS {
            return Err(ContractError::Capacity);
        }
        self.staged.push(StagedCandidate {
            content_id: candidate_id.into(),
            validation: None,
        });
        Ok(())
    }

    pub fn record_validation(
        &mut self,
        grant: &TaskGrantV0,
        evidence: ValidationEvidenceV0,
        now_ms: u64,
    ) -> Result<(), ContractError> {
        self.authorize(grant, now_ms)?;
        self.check_binding(&evidence)?;
        let entry = self
            .staged
            .iter_mut()
            .find(|entry| entry.content_id == evidence.candidate_id)
            .ok_or(ContractError::NotStaged)?;
        // Invalid/failed outcomes replace earlier observations; they never
        // change the accepted output or leave an earlier success usable.
        entry.validation = Some(evidence);
        Ok(())
    }

    fn check_binding(&self, evidence: &ValidationEvidenceV0) -> Result<(), ContractError> {
        valid_id(&evidence.candidate_id)?;
        if evidence.task_id != self.contract.task_id
            || evidence.domain_id != self.contract.domain_id
            || evidence.resource != self.contract.resource
            || evidence.grant_generation != self.generation
            || evidence.schema_id != self.contract.schema_id
            || evidence.policy_id != self.contract.policy_id
            || evidence.validator_id != self.contract.validator_id
        {
            return Err(ContractError::ValidationMismatch);
        }
        Ok(())
    }

    pub fn commit(
        &mut self,
        grant: &TaskGrantV0,
        request: &CommitRequestV0,
        now_ms: u64,
    ) -> Result<CommitReceiptV0, ContractError> {
        self.authorize(grant, now_ms)?;
        if request.task_id != self.contract.task_id
            || request.domain_id != self.contract.domain_id
            || request.resource != self.contract.resource
        {
            return Err(ContractError::WrongAuthority);
        }
        if request.request_id == 0 {
            return Err(ContractError::InvalidContract);
        }
        valid_id(&request.candidate_id)?;
        valid_id(&request.expected_content_id)?;
        if let Some((original, receipt)) = self
            .receipts
            .iter()
            .find(|(original, _)| original.request_id == request.request_id)
        {
            return if original == request {
                Ok(receipt.clone())
            } else {
                Err(ContractError::RequestReuse)
            };
        }
        if request.expected_revision != self.accepted.revision
            || request.expected_content_id != self.accepted.content_id
        {
            return Err(ContractError::Conflict);
        }
        let entry = self
            .staged
            .iter()
            .find(|entry| entry.content_id == request.candidate_id)
            .ok_or(ContractError::NotStaged)?;
        let evidence = entry
            .validation
            .as_ref()
            .ok_or(ContractError::InvalidCandidate)?;
        self.check_binding(evidence)?;
        if now_ms >= evidence.valid_until_ms {
            return Err(ContractError::Expired);
        }
        let budget = &self.contract.execution_budget;
        if evidence.outcome == ValidationOutcomeV0::Timeout
            || evidence.wall_elapsed_ms > budget.wall_ms
            || evidence.guest_elapsed_ms > budget.guest_ms
            || evidence.guest_elapsed_ms > evidence.wall_elapsed_ms
            || evidence.diagnostics_bytes > budget.max_diagnostics_bytes
            || evidence.diagnostics_truncated
        {
            return Err(ContractError::ExecutionLimit);
        }
        if evidence.outcome != ValidationOutcomeV0::Valid {
            return Err(ContractError::InvalidCandidate);
        }
        if self.receipts.len() == MAX_RECORDS {
            return Err(ContractError::Capacity);
        }
        let revision = self
            .accepted
            .revision
            .checked_add(1)
            .ok_or(ContractError::RevisionExhausted)?;
        let receipt = CommitReceiptV0 {
            request_id: request.request_id,
            task_id: self.contract.task_id,
            domain_id: self.contract.domain_id,
            resource: self.contract.resource.clone(),
            grant_generation: self.generation,
            prior: self.accepted.clone(),
            accepted: OutputRevisionV0 {
                revision,
                content_id: request.candidate_id.clone(),
            },
        };
        self.receipts.push((request.clone(), receipt.clone()));
        self.accepted = receipt.accepted.clone();
        Ok(receipt)
    }
}

/// Private supervisor/worker IPC, never an agent-facing validation attestation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidatorJobV0 {
    pub schema_version: u32,
    pub store_root: String,
    pub validator_id: String,
    pub candidate_id: String,
    pub schema_id: String,
    pub budget: ExecutionBudgetV0,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidatorResultV0 {
    pub schema_version: u32,
    pub outcome: ValidationOutcomeV0,
    pub diagnostics: Vec<u8>,
    pub truncated: bool,
    pub guest_elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskPolicyV1 {
    pub schema_version: u32,
    pub task_id: u64,
    pub domain_id: u64,
    pub resource_id: u64,
    pub allowed_rights: u32,
    pub max_grant_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskSnapshotV1 {
    pub schema_version: u32,
    pub task_id: u64,
    pub resource_id: u64,
    pub revision: u64,
    pub content_id: String,
    pub grant_generation: u64,
    pub granted_rights: u32,
    pub grant_expires_at_ms: u64,
    pub now_ms: u64,
    pub schema_id: String,
    pub policy_id: String,
    pub validator_id: String,
    pub input_resources: [u64; 3],
    pub validation: Option<ValidationEvidenceV0>,
    pub validation_current: bool,
}
