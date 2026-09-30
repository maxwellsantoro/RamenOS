//! A0 contract fixtures: a reference model, not service/kernel enforcement.
use artifact_store_schema::agent_task::{
    CommitRequestV0, ContractError, GrantKindV0, TaskContractV0, TaskGrantV0, TaskStateV0,
    ValidationEvidenceV0, ValidationOutcomeV0,
};

fn id(digit: char) -> String {
    format!("sha256:{}", digit.to_string().repeat(64))
}

fn fixture() -> (TaskStateV0, TaskGrantV0) {
    let contract: TaskContractV0 =
        serde_json::from_str(include_str!("fixtures/agent_task_contract_v0.json")).unwrap();
    let grant = TaskGrantV0 {
        task_id: contract.task_id,
        domain_id: contract.domain_id,
        resource: contract.resource.clone(),
        generation: 1,
        expires_at_ms: 100_000,
        kind: GrantKindV0::Mutation,
    };
    (TaskStateV0::new(contract, &id('a')).unwrap(), grant)
}

fn validation(state: &TaskStateV0, grant: &TaskGrantV0, candidate: &str) -> ValidationEvidenceV0 {
    let contract = state.contract();
    ValidationEvidenceV0 {
        task_id: contract.task_id,
        domain_id: contract.domain_id,
        resource: contract.resource.clone(),
        grant_generation: grant.generation,
        candidate_id: candidate.into(),
        schema_id: contract.schema_id.clone(),
        policy_id: contract.policy_id.clone(),
        validator_id: contract.validator_id.clone(),
        valid_until_ms: 90_000,
        wall_elapsed_ms: 100,
        guest_elapsed_ms: 50,
        diagnostics_bytes: 32,
        diagnostics_truncated: false,
        outcome: ValidationOutcomeV0::Valid,
    }
}

fn request(state: &TaskStateV0, candidate: &str, request_id: u64) -> CommitRequestV0 {
    CommitRequestV0 {
        request_id,
        task_id: state.contract().task_id,
        domain_id: state.contract().domain_id,
        resource: state.contract().resource.clone(),
        candidate_id: candidate.into(),
        expected_revision: state.accepted().revision,
        expected_content_id: state.accepted().content_id.clone(),
    }
}

fn ready(state: &mut TaskStateV0, grant: &TaskGrantV0, candidate: &str) {
    state.stage(grant, candidate, 10).unwrap();
    state
        .record_validation(grant, validation(state, grant, candidate), 20)
        .unwrap();
}

#[test]
fn stage_and_validation_do_not_publish_output() {
    let (mut state, grant) = fixture();
    let before = state.accepted().clone();
    ready(&mut state, &grant, &id('b'));
    assert_eq!(state.accepted(), &before);
    let commit = request(&state, &id('b'), 1);
    let receipt = state.commit(&grant, &commit, 30).unwrap();
    assert_eq!(receipt.task_id, grant.task_id);
    assert_eq!(receipt.domain_id, grant.domain_id);
    assert_eq!(receipt.resource, grant.resource);
    assert_eq!(receipt.prior, before);
    assert_eq!(receipt.accepted.content_id, id('b'));
    assert_eq!(receipt.accepted.revision, before.revision + 1);
    assert_eq!(state.accepted(), &receipt.accepted);
}

#[test]
fn invalid_and_unvalidated_candidates_cannot_publish() {
    let (mut state, grant) = fixture();
    let before = state.accepted().clone();
    let candidate = id('b');
    let commit = request(&state, &candidate, 1);
    assert_eq!(
        state.commit(&grant, &commit, 30),
        Err(ContractError::NotStaged)
    );
    state.stage(&grant, &candidate, 10).unwrap();
    assert_eq!(
        state.commit(&grant, &commit, 30),
        Err(ContractError::InvalidCandidate)
    );
    let mut evidence = validation(&state, &grant, &candidate);
    evidence.outcome = ValidationOutcomeV0::Invalid;
    state.record_validation(&grant, evidence, 20).unwrap();
    assert_eq!(
        state.commit(&grant, &commit, 30),
        Err(ContractError::InvalidCandidate)
    );
    assert_eq!(state.accepted(), &before);
}

#[test]
fn validation_is_bound_to_task_domain_resource_candidate_and_pins() {
    for field in 0..8 {
        let (mut state, grant) = fixture();
        state.stage(&grant, &id('b'), 10).unwrap();
        let mut evidence = validation(&state, &grant, &id('b'));
        match field {
            0 => evidence.task_id += 1,
            1 => evidence.domain_id += 1,
            2 => evidence.resource = "workspace:B/config".into(),
            3 => evidence.candidate_id = id('c'),
            4 => evidence.schema_id = id('c'),
            5 => evidence.policy_id = id('c'),
            6 => evidence.validator_id = id('c'),
            7 => evidence.grant_generation += 1,
            _ => unreachable!(),
        }
        assert!(state.record_validation(&grant, evidence, 20).is_err());
        let commit = request(&state, &id('b'), 1);
        assert_eq!(
            state.commit(&grant, &commit, 30),
            Err(ContractError::InvalidCandidate)
        );
        assert_eq!(state.accepted().content_id, id('a'));
    }
}

#[test]
fn wrong_authority_and_revocation_block_every_mutation() {
    for field in 0..6 {
        let (mut state, grant) = fixture();
        ready(&mut state, &grant, &id('b'));
        let mut wrong = grant.clone();
        match field {
            0 => wrong.task_id += 1,
            1 => wrong.domain_id += 1,
            2 => wrong.resource = "workspace:B/config".into(),
            3 => wrong.kind = GrantKindV0::Read,
            4 => wrong.expires_at_ms = 30,
            5 => {
                state.revoke().unwrap();
            }
            _ => unreachable!(),
        }
        assert!(state.stage(&wrong, &id('c'), 30).is_err());
        let evidence = validation(&state, &wrong, &id('b'));
        assert!(state.record_validation(&wrong, evidence, 30).is_err());
        let commit = request(&state, &id('b'), 1);
        assert!(state.commit(&wrong, &commit, 30).is_err());
        assert_eq!(state.accepted().content_id, id('a'));
    }
}

#[test]
fn renewing_authority_requires_fresh_validation_after_revocation() {
    let (mut state, mut grant) = fixture();
    ready(&mut state, &grant, &id('b'));
    state.revoke().unwrap();
    grant.generation += 1;
    let commit = request(&state, &id('b'), 1);
    assert_eq!(
        state.commit(&grant, &commit, 30),
        Err(ContractError::ValidationMismatch)
    );
    let evidence = validation(&state, &grant, &id('b'));
    state.record_validation(&grant, evidence, 40).unwrap();
    assert!(state.commit(&grant, &commit, 50).is_ok());
}

#[test]
fn concurrent_commits_and_aba_revisions_fail_closed() {
    let (mut state, grant) = fixture();
    ready(&mut state, &grant, &id('b'));
    ready(&mut state, &grant, &id('c'));
    let first = request(&state, &id('b'), 1);
    let stale = request(&state, &id('c'), 2);
    state.commit(&grant, &first, 30).unwrap();
    assert_eq!(
        state.commit(&grant, &stale, 40),
        Err(ContractError::Conflict)
    );
    ready(&mut state, &grant, &id('a'));
    let back_to_a = request(&state, &id('a'), 3);
    state.commit(&grant, &back_to_a, 50).unwrap();
    assert_eq!(state.accepted().content_id, stale.expected_content_id);
    assert_eq!(
        state.commit(&grant, &stale, 60),
        Err(ContractError::Conflict)
    );
    assert_eq!(state.accepted().revision, 2);
}

#[test]
fn lost_reply_retry_returns_original_receipt_without_second_effect() {
    let (mut state, grant) = fixture();
    ready(&mut state, &grant, &id('b'));
    let commit = request(&state, &id('b'), 1);
    let receipt = state.commit(&grant, &commit, 30).unwrap();
    ready(&mut state, &grant, &id('c'));
    let next = request(&state, &id('c'), 2);
    state.commit(&grant, &next, 40).unwrap();
    assert_eq!(state.commit(&grant, &commit, 50).unwrap(), receipt);
    assert_eq!(state.accepted().content_id, id('c'));
    assert_eq!(state.accepted().revision, 2);
    let mut altered = commit;
    altered.candidate_id = id('c');
    assert_eq!(
        state.commit(&grant, &altered, 60),
        Err(ContractError::RequestReuse)
    );
}

#[test]
fn invalidated_validation_never_reuses_an_earlier_success() {
    let (mut state, grant) = fixture();
    ready(&mut state, &grant, &id('b'));
    let mut evidence = validation(&state, &grant, &id('b'));
    evidence.outcome = ValidationOutcomeV0::HostFailure;
    state.record_validation(&grant, evidence, 20).unwrap();
    let commit = request(&state, &id('b'), 1);
    assert_eq!(
        state.commit(&grant, &commit, 30),
        Err(ContractError::InvalidCandidate)
    );
}

#[test]
fn expired_and_over_budget_validation_cannot_commit() {
    for fault in 0..6 {
        let (mut state, grant) = fixture();
        state.stage(&grant, &id('b'), 10).unwrap();
        let mut evidence = validation(&state, &grant, &id('b'));
        match fault {
            0 => evidence.valid_until_ms = 30,
            1 => evidence.wall_elapsed_ms = 35_001,
            2 => evidence.guest_elapsed_ms = 30_001,
            3 => evidence.diagnostics_bytes = 4097,
            4 => evidence.outcome = ValidationOutcomeV0::Timeout,
            5 => evidence.diagnostics_truncated = true,
            _ => unreachable!(),
        }
        state.record_validation(&grant, evidence, 20).unwrap();
        let commit = request(&state, &id('b'), 1);
        assert!(state.commit(&grant, &commit, 30).is_err());
        assert_eq!(state.accepted().content_id, id('a'));
    }
}

#[test]
fn malformed_contract_and_budget_fail_closed() {
    for fault in 0..9 {
        let (state, _) = fixture();
        let mut contract = state.contract().clone();
        match fault {
            0 => contract.schema_version = 2,
            1 => contract.task_id = 0,
            2 => contract.domain_id = 0,
            3 => contract.resource.clear(),
            4 => contract.validator_id = "sha256:../escape".into(),
            5 => contract.execution_budget.guest_ms = 0,
            6 => contract.execution_budget.wall_ms = 10,
            7 => contract.execution_budget.host_call_ms = 35_001,
            8 => contract.execution_budget.max_diagnostics_bytes = 65_537,
            _ => unreachable!(),
        }
        assert!(TaskStateV0::new(contract, &id('a')).is_err());
    }
    let (mut state, grant) = fixture();
    assert_eq!(
        state.stage(&grant, "bad-id", 10),
        Err(ContractError::InvalidContentId)
    );
    let mut json: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/agent_task_contract_v0.json")).unwrap();
    json["allow_all"] = true.into();
    assert!(serde_json::from_value::<TaskContractV0>(json).is_err());
}

#[test]
fn staged_candidates_and_receipts_are_bounded_without_eviction() {
    let (mut state, grant) = fixture();
    for n in 1..=64 {
        let candidate = format!("sha256:{n:064x}");
        ready(&mut state, &grant, &candidate);
        let commit = request(&state, &candidate, n);
        assert!(state.commit(&grant, &commit, 30).is_ok());
    }
    assert_eq!(
        state.stage(&grant, &id('f'), 40),
        Err(ContractError::Capacity)
    );
    assert_eq!(state.accepted().revision, 64);

    // Reusing one staged identity cannot bypass the independent receipt bound.
    let (mut state, grant) = fixture();
    ready(&mut state, &grant, &id('b'));
    for n in 1..=64 {
        let commit = request(&state, &id('b'), n);
        assert!(state.commit(&grant, &commit, 30).is_ok());
    }
    let overflow = request(&state, &id('b'), 65);
    assert_eq!(
        state.commit(&grant, &overflow, 40),
        Err(ContractError::Capacity)
    );
    assert_eq!(state.accepted().revision, 64);
}

#[test]
fn receipt_retrieval_requires_current_authority_and_exact_request_scope() {
    let (mut state, mut grant) = fixture();
    ready(&mut state, &grant, &id('b'));
    let commit = request(&state, &id('b'), 1);
    let receipt = state.commit(&grant, &commit, 30).unwrap();
    for field in 0..4 {
        let mut wrong = commit.clone();
        match field {
            0 => wrong.task_id += 1,
            1 => wrong.domain_id += 1,
            2 => wrong.resource = "workspace:B/config".into(),
            3 => wrong.request_id = 0,
            _ => unreachable!(),
        }
        assert!(state.commit(&grant, &wrong, 40).is_err());
    }
    state.revoke().unwrap();
    assert_eq!(
        state.commit(&grant, &commit, 50),
        Err(ContractError::Revoked)
    );
    grant.generation += 1;
    assert_eq!(state.commit(&grant, &commit, 60).unwrap(), receipt);
    assert_eq!(state.accepted().revision, 1);
}
