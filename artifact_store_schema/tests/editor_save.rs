//! Gate-first UI1.1b pure records. No IO, clock or grant is supplied by a schema.
use artifact_store_schema::editor_save::*;
use sha2::{Digest, Sha256};

fn hash(bytes: &[u8]) -> Hash32 {
    Sha256::digest(bytes).into()
}
fn actor() -> EditorActorV0 {
    EditorActorV0::try_new(7, 11, 1, 21, 1).unwrap()
}
fn selected(revision: u64, bytes: &[u8]) -> SelectedArtifactV0 {
    SelectedArtifactV0::try_new(7, 31, 1, revision, hash(bytes), bytes.len() as u32, [9; 32])
        .unwrap()
}
fn binding(operation: u64, revision: u64) -> EditorSaveBindingV0 {
    let allocation =
        EditorSaveAllocationV0::try_new(actor(), &selected(revision, b"old\n"), operation).unwrap();
    let source = EditorSourceBindingV0::try_new(
        0x0200_0000_0000_0001 | (operation << 32),
        1,
        4,
        hash(b"new\n"),
    )
    .unwrap();
    EditorSaveBindingV0::try_new(allocation, source).unwrap()
}
fn allocated(operation: u64) -> EditorOperationRecordV0 {
    EditorOperationRecordV0 {
        allocation: binding(operation, 1).allocation,
        allocated_service_epoch: 1,
        binding: None,
        submitted_service_epoch: None,
        permit: None,
        closure: EditorClosureV0::None,
        state: EditorOperationStateV0::Allocated,
        successor: None,
        receipt: None,
    }
}
fn submitted(operation: u64, closure: EditorClosureV0) -> EditorOperationRecordV0 {
    let mut record = allocated(operation);
    record.binding = Some(binding(operation, 1));
    record.submitted_service_epoch = Some(1);
    record.state = EditorOperationStateV0::Submitted;
    record.closure = closure;
    record
}
fn permitted(operation: u64, closure: EditorClosureV0) -> EditorOperationRecordV0 {
    let mut record = submitted(operation, closure);
    record.permit =
        Some(EditorCommitPermitV0::try_new(binding(operation, 1), 1, operation, 1).unwrap());
    record.state = EditorOperationStateV0::Permitted;
    record
}
fn journal(records: Vec<EditorOperationRecordV0>) -> EditorSelectionJournalV0 {
    let mut journal = EditorSelectionJournalV0::try_new(selected(1, b"old\n"), [5; 32]).unwrap();
    journal.operation_high_water = 16;
    journal.writer_high_water = 16;
    journal.transition_sequence = 32;
    journal.prior_journal_hash = [8; 32];
    journal.operations = BoundedVec::try_from_vec(records).unwrap();
    journal
}

#[test]
fn editor_save_identity_binding_and_bounds_are_explicit() {
    assert_eq!(
        (
            VERSION,
            TEXT_HEADER_LEN,
            RECEIPT_LEN,
            MAX_TEXT_LEN,
            MAX_OPERATIONS
        ),
        (1, 64, 176, 4096, 16)
    );
    let valid = binding(1, 1);
    valid.validate().unwrap();
    let mut bad_version = valid.clone();
    bad_version.schema_version = 2;
    assert!(bad_version.validate().is_err());
    for field in 0..5 {
        let mut bad = actor();
        match field {
            0 => bad.owner_id = 0,
            1 => bad.session_id = 0,
            2 => bad.session_generation = 0,
            3 => bad.instance_id = 0,
            _ => bad.instance_generation = 0,
        }
        assert!(bad.validate().is_err());
    }
    for field in 0..4 {
        let mut bad = valid.clone();
        match field {
            0 => bad.allocation.selected_object_id = 0,
            1 => bad.allocation.selected_generation = 0,
            2 => bad.allocation.operation_id = 0,
            _ => bad.allocation.expected_revision = 0,
        }
        assert!(bad.validate().is_err());
    }
    assert!(EditorSourceBindingV0::try_new(1, u64::from(u32::MAX) + 1, 4, [0; 32]).is_err());
    assert!(EditorSourceBindingV0::try_new(1, 1, 4097, [0; 32]).is_err());
    assert!(SelectedArtifactV0::try_new(7, 31, 1, 1, [0; 32], 4097, [0; 32]).is_err());
    let zero_digest = SelectedArtifactV0::try_new(7, 31, 1, 1, [0; 32], 0, [0; 32]).unwrap();
    assert_eq!(
        zero_digest.content_id().unwrap().as_str(),
        format!("sha256:{}", "0".repeat(64))
    );
}

#[test]
fn editor_text_header_has_exact_le_layout_and_ascii_consumption() {
    for bytes in [b"".as_slice(), b"A\tB\n!", &vec![b'x'; 4096]] {
        let header = EditorTextHeaderV0::try_new(31, 9, bytes).unwrap();
        let wire = header.encode_le().unwrap();
        assert_eq!(wire.len(), 64);
        assert_eq!(&wire[0..4], &1u32.to_le_bytes());
        assert_eq!(&wire[4..8], &(64u32 + bytes.len() as u32).to_le_bytes());
        assert_eq!(&wire[8..16], &31u64.to_le_bytes());
        assert_eq!(&wire[16..24], &9u64.to_le_bytes());
        assert_eq!(&wire[24..56], &hash(bytes));
        assert_eq!(&wire[56..60], &(bytes.len() as u32).to_le_bytes());
        assert_eq!(&wire[60..64], &[0; 4]);
        let decoded = EditorTextHeaderV0::decode_le(&wire).unwrap();
        decoded.validate_bytes(bytes).unwrap();
        let mut changed = bytes.to_vec();
        changed.push(b'x');
        assert!(decoded.validate_bytes(&changed).is_err());
    }
    for bytes in [
        &[0u8][..],
        &[127][..],
        &[255][..],
        &[13][..],
        &vec![b'x'; 4097],
    ] {
        assert!(EditorTextHeaderV0::try_new(31, 9, bytes).is_err());
    }
    let wire = EditorTextHeaderV0::try_new(31, 9, b"old\n")
        .unwrap()
        .encode_le()
        .unwrap();
    assert!(EditorTextHeaderV0::decode_le(&wire[..63]).is_err());
    let mut oversized = wire.to_vec();
    oversized.push(0);
    assert!(EditorTextHeaderV0::decode_le(&oversized).is_err());
    for offset in [0, 4, 56, 60] {
        let mut bad = wire;
        bad[offset..offset + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(
            EditorTextHeaderV0::decode_le(&bad)
                .and_then(|h| h.validate_bytes(b"old\n"))
                .is_err()
        );
    }
    for offset in [8, 16] {
        let mut bad = wire;
        bad[offset..offset + 8].copy_from_slice(&0u64.to_le_bytes());
        assert!(
            EditorTextHeaderV0::decode_le(&bad)
                .and_then(|h| h.validate_bytes(b"old\n"))
                .is_err()
        );
    }
    let max_header = EditorTextHeaderV0::try_new(u64::MAX, u64::MAX, b"old\n").unwrap();
    EditorTextHeaderV0::decode_le(&max_header.encode_le().unwrap())
        .unwrap()
        .validate_bytes(b"old\n")
        .unwrap();
    let mut bad = wire;
    bad[24] ^= 1;
    assert!(
        EditorTextHeaderV0::decode_le(&bad)
            .and_then(|h| h.validate_bytes(b"old\n"))
            .is_err()
    );
}

#[test]
fn editor_receipt_exact_176_bytes_and_every_bound_identity() {
    let binding = binding(3, 1);
    let permit = EditorCommitPermitV0::try_new(binding.clone(), 4, 5, 6).unwrap();
    let receipt = EditorSaveReceiptV0::committed(&permit, 100).unwrap();
    let wire = receipt.encode_le().unwrap();
    for (offset, value) in [(0, 1u32), (4, 176), (8, 0), (12, 0)] {
        assert_eq!(&wire[offset..offset + 4], &value.to_le_bytes());
    }
    let values = [7u64, 11, 1, 21, 1, 31, 1, 3, 1, 2, 4, 100];
    for (i, value) in values.iter().enumerate() {
        assert_eq!(&wire[16 + i * 8..24 + i * 8], &value.to_le_bytes());
    }
    assert_eq!(&wire[112..144], &hash(b"old\n"));
    assert_eq!(&wire[144..176], &hash(b"new\n"));
    assert_eq!(EditorSaveReceiptV0::decode_le(&wire).unwrap(), receipt);
    receipt.validate_binding(&binding).unwrap();
    for offset in (16..88).step_by(8).chain([112, 144]) {
        let mut changed = wire;
        changed[offset] ^= 1;
        assert!(
            EditorSaveReceiptV0::decode_le(&changed)
                .and_then(|r| r.validate_binding(&binding))
                .is_err()
        );
    }
    for offset in [0, 4, 8, 12] {
        let mut bad = wire;
        bad[offset..offset + 4].copy_from_slice(&99u32.to_le_bytes());
        assert!(EditorSaveReceiptV0::decode_le(&bad).is_err());
    }
    assert!(EditorSaveReceiptV0::decode_le(&wire[..175]).is_err());
    assert!(EditorSaveReceiptV0::decode_le(&[wire.as_slice(), &[0]].concat()).is_err());
    let mut bad = receipt.clone();
    bad.result_revision = 3;
    assert!(bad.validate().is_err());
    let noncommit = EditorSaveReceiptV0::noncommit(&binding, 4).unwrap();
    assert_eq!(
        (noncommit.result_revision, noncommit.committed_at_ms),
        (0, 0)
    );
    assert_eq!(serde_json::to_value(receipt.outcome).unwrap(), "committed");
    assert_eq!(
        serde_json::to_value(noncommit.outcome).unwrap(),
        "definitive_noncommit"
    );
    for raw in ["\"Committed\"", "\"definitiveNoncommit\"", "0", "2"] {
        assert!(serde_json::from_str::<EditorReceiptOutcomeV0>(raw).is_err());
    }
}

#[test]
fn editor_permit_payload_cannot_wrap_successor_or_change_binding() {
    let valid = EditorCommitPermitV0::try_new(binding(1, 1), 1, 1, 1).unwrap();
    valid.validate().unwrap();
    assert_eq!(valid.successor_revision, 2);
    assert!(EditorCommitPermitV0::try_new(binding(1, u64::MAX), 1, 1, 1).is_err());
    for field in 0..4 {
        let mut bad = valid.clone();
        match field {
            0 => bad.service_epoch = 0,
            1 => bad.writer_id = 0,
            2 => bad.admission_epoch = 0,
            _ => bad.successor_revision = 3,
        }
        assert!(bad.validate().is_err());
    }
    let mut changed = valid.binding.clone();
    changed.source.content_hash[0] ^= 1;
    assert!(
        EditorSaveReceiptV0::committed(&valid, 1)
            .unwrap()
            .validate_binding(&changed)
            .is_err()
    );
}

#[test]
fn editor_journal_canonical_digest_and_json_denials() {
    let journal = journal(vec![allocated(1)]);
    let bytes = journal.canonical_bytes().unwrap();
    assert_eq!(journal.digest().unwrap(), hash(&bytes));
    assert_eq!(
        EditorSelectionJournalV0::decode_canonical(&bytes)
            .unwrap()
            .canonical_bytes()
            .unwrap(),
        bytes
    );
    assert_eq!(journal.canonical_bytes().unwrap(), bytes);
    // Independent golden encoder: no serialization of the object under test.
    let array = |values: &[u8]| {
        format!(
            "[{}]",
            values
                .iter()
                .map(u8::to_string)
                .collect::<Vec<_>>()
                .join(",")
        )
    };
    let old = array(&hash(b"old\n"));
    let nine = array(&[9; 32]);
    let five = array(&[5; 32]);
    let eight = array(&[8; 32]);
    let selected = format!(
        "{{\"schema_version\":1,\"owner_id\":7,\"selected_object_id\":31,\"selected_generation\":1,\"revision\":1,\"content_hash\":{old},\"byte_len\":4,\"manifest_hash\":{nine}}}"
    );
    let allocation = format!(
        "{{\"schema_version\":1,\"actor\":{{\"owner_id\":7,\"session_id\":11,\"session_generation\":1,\"instance_id\":21,\"instance_generation\":1}},\"selected_object_id\":31,\"selected_generation\":1,\"operation_id\":1,\"expected_revision\":1,\"expected_content_hash\":{old}}}"
    );
    let record = format!(
        "{{\"allocation\":{allocation},\"allocated_service_epoch\":1,\"binding\":null,\"submitted_service_epoch\":null,\"permit\":null,\"closure\":\"none\",\"state\":\"allocated\",\"successor\":null,\"receipt\":null}}"
    );
    let golden = format!(
        "{{\"schema_version\":1,\"owner_id\":7,\"selected_object_id\":31,\"selected_generation\":1,\"signing_policy_hash\":{five},\"initial\":{selected},\"current\":{selected},\"service_epoch\":1,\"operation_high_water\":16,\"writer_high_water\":16,\"transition_sequence\":32,\"prior_journal_hash\":{eight},\"operations\":[{record}]}}"
    );
    assert_eq!(
        bytes,
        golden.as_bytes(),
        "compact declaration order, explicit nulls and32-integer hashes"
    );
    let reordered = golden.replacen(
        "\"schema_version\":1,\"owner_id\":7",
        "\"owner_id\":7,\"schema_version\":1",
        1,
    );
    let missing_null = golden.replacen(",\"binding\":null", "", 1);
    let wrong_hash = golden.replacen(
        &format!("\"content_hash\":{old}"),
        "\"content_hash\":[0]",
        1,
    );
    let nested_duplicate = golden.replacen("\"actor\":{", "\"actor\":{\"owner_id\":7,", 1);
    let nested_unknown = golden.replacen("\"actor\":{", "\"actor\":{\"unknown\":0,", 1);
    for bad in [
        reordered,
        missing_null,
        wrong_hash,
        nested_duplicate,
        nested_unknown,
    ] {
        assert_ne!(bad, golden);
        assert!(EditorSelectionJournalV0::decode_canonical(bad.as_bytes()).is_err());
    }
    let raw = String::from_utf8(bytes.clone()).unwrap();
    let duplicate = raw.replacen("{", "{\"schema_version\":1,", 1);
    let extra = raw.replacen("{", "{\"unknown\":0,", 1);
    let version = raw.replacen("\"schema_version\":1", "\"schema_version\":2", 1);
    let float = raw.replacen("\"service_epoch\":1", "\"service_epoch\":1.0", 1);
    let negative = raw.replacen("\"service_epoch\":1", "\"service_epoch\":-1", 1);
    let overflow = raw.replacen(
        "\"service_epoch\":1",
        "\"service_epoch\":18446744073709551616",
        1,
    );
    for bad in [
        duplicate,
        extra,
        version,
        float,
        negative,
        overflow,
        format!(" {raw}"),
        format!("{raw}null"),
    ] {
        assert_ne!(bad, raw);
        assert!(EditorSelectionJournalV0::decode_canonical(bad.as_bytes()).is_err());
    }
    assert!(EditorSelectionJournalV0::decode_canonical(&bytes[..bytes.len() - 1]).is_err());
    assert!(
        EditorSelectionJournalV0::decode_canonical(&vec![b' '; MAX_JOURNAL_BYTES + 1]).is_err()
    );
}

#[test]
fn editor_journal_retains_sixteen_allocation_records_and_rejects_seventeenth() {
    let journal = journal((1..=16).map(allocated).collect());
    journal.validate().unwrap();
    assert_eq!(journal.operations.len(), 16);
    assert!(
        BoundedVec::<EditorOperationRecordV0, 16>::try_from_vec((1..=17).map(allocated).collect())
            .is_err()
    );
    let mut bounded =
        BoundedVec::<EditorOperationRecordV0, 16>::try_from_vec((1..=16).map(allocated).collect())
            .unwrap();
    assert!(bounded.try_push(allocated(17)).is_err());
    assert_eq!(bounded.len(), 16);
    let mut raw = serde_json::to_value(&journal).unwrap();
    raw["operations"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::to_value(allocated(17)).unwrap());
    assert!(
        EditorSelectionJournalV0::decode_canonical(&serde_json::to_vec(&raw).unwrap()).is_err()
    );
    assert!(serde_json::from_value::<EditorSelectionJournalV0>(raw).is_err());
}

#[test]
fn editor_journal_active_reservation_counts_submitted_and_retired_permits() {
    journal(vec![submitted(1, EditorClosureV0::None)])
        .validate()
        .unwrap();
    assert!(
        journal(vec![
            submitted(1, EditorClosureV0::None),
            submitted(2, EditorClosureV0::None)
        ])
        .validate()
        .is_err()
    );
    assert!(
        journal(vec![
            permitted(1, EditorClosureV0::Revoked),
            submitted(2, EditorClosureV0::None)
        ])
        .validate()
        .is_err()
    );
    assert!(
        journal(vec![
            permitted(1, EditorClosureV0::Deadline),
            permitted(2, EditorClosureV0::ServiceRetired)
        ])
        .validate()
        .is_err()
    );
    journal(vec![
        submitted(1, EditorClosureV0::Deadline),
        submitted(2, EditorClosureV0::None),
    ])
    .validate()
    .unwrap();
    journal(vec![permitted(1, EditorClosureV0::Revoked)])
        .validate()
        .unwrap();
}

#[test]
fn editor_journal_selection_and_receipt_are_one_replayed_transition() {
    let permit = EditorCommitPermitV0::try_new(binding(1, 1), 1, 1, 1).unwrap();
    let mut record = permitted(1, EditorClosureV0::Revoked);
    record.state = EditorOperationStateV0::Committed;
    record.receipt = Some(EditorSaveReceiptV0::committed(&permit, 9).unwrap());
    record.successor = Some(selected(2, b"new\n"));
    let mut valid = journal(vec![record.clone()]);
    valid.current = selected(2, b"new\n");
    valid.validate().unwrap();
    for field in 0..7 {
        let mut bad = valid.clone();
        let mut records = bad.operations.as_slice().to_vec();
        match field {
            0 => bad.current = selected(1, b"old\n"),
            1 => records[0].receipt = None,
            2 => records[0].permit = None,
            3 => records[0].successor = None,
            4 => records[0].receipt.as_mut().unwrap().source_content_hash[0] ^= 1,
            5 => records[0].successor.as_mut().unwrap().revision = 3,
            _ => records[0].allocation.expected_content_hash[0] ^= 1,
        }
        bad.operations = BoundedVec::try_from_vec(records).unwrap();
        assert!(bad.validate().is_err());
    }
    // A second internally coherent commit must extend the first selected identity.
    let second_base = selected(2, b"new\n");
    let second_allocation = EditorSaveAllocationV0::try_new(actor(), &second_base, 2).unwrap();
    let second_source =
        EditorSourceBindingV0::try_new(0x0200_0002_0000_0001, 1, 6, hash(b"third\n")).unwrap();
    let second_binding = EditorSaveBindingV0::try_new(second_allocation, second_source).unwrap();
    let second_permit = EditorCommitPermitV0::try_new(second_binding.clone(), 1, 2, 1).unwrap();
    let mut second = allocated(2);
    second.allocation = second_binding.allocation.clone();
    second.binding = Some(second_binding);
    second.submitted_service_epoch = Some(1);
    second.permit = Some(second_permit.clone());
    second.state = EditorOperationStateV0::Committed;
    second.successor = Some(selected(3, b"third\n"));
    second.receipt = Some(EditorSaveReceiptV0::committed(&second_permit, 10).unwrap());
    let mut two = journal(vec![record.clone(), second]);
    two.current = selected(3, b"third\n");
    two.validate().unwrap();
    for fork_base in [selected(1, b"old\n"), selected(3, b"new\n")] {
        let a = EditorSaveAllocationV0::try_new(actor(), &fork_base, 2).unwrap();
        let source =
            EditorSourceBindingV0::try_new(0x0200_0002_0000_0001, 1, 6, hash(b"third\n")).unwrap();
        let b = EditorSaveBindingV0::try_new(a, source).unwrap();
        let p = EditorCommitPermitV0::try_new(b.clone(), 1, 2, 1).unwrap();
        p.validate().unwrap();
        let r = EditorSaveReceiptV0::committed(&p, 10).unwrap();
        r.validate_binding(&b).unwrap();
        let successor = selected(fork_base.revision + 1, b"third\n");
        successor.validate().unwrap();
        let mut fork = allocated(2);
        fork.allocation = b.allocation.clone();
        fork.binding = Some(b);
        fork.submitted_service_epoch = Some(1);
        fork.permit = Some(p);
        fork.state = EditorOperationStateV0::Committed;
        fork.receipt = Some(r);
        fork.successor = Some(successor.clone());
        let mut bad = journal(vec![record.clone(), fork]);
        bad.current = successor;
        assert!(
            bad.validate().is_err(),
            "coherent fork or gap must fail aggregate replay"
        );
    }
    let binding = binding(2, 1);
    let mut closed = submitted(2, EditorClosureV0::Deadline);
    closed.state = EditorOperationStateV0::Noncommit;
    closed.receipt = Some(EditorSaveReceiptV0::noncommit(&binding, 1).unwrap());
    journal(vec![closed]).validate().unwrap();
}

#[test]
fn editor_journal_counter_identity_and_state_shape_fail_closed() {
    let valid = journal(vec![allocated(1)]);
    for field in 0..8 {
        let mut bad = valid.clone();
        let mut records = bad.operations.as_slice().to_vec();
        match field {
            0 => bad.owner_id = 0,
            1 => bad.selected_object_id += 1,
            2 => bad.selected_generation += 1,
            3 => bad.service_epoch = 0,
            4 => bad.operation_high_water = 0,
            5 => bad.transition_sequence = 0,
            6 => records[0].state = EditorOperationStateV0::Committed,
            _ => records[0].allocated_service_epoch = 0,
        }
        bad.operations = BoundedVec::try_from_vec(records).unwrap();
        assert!(bad.validate().is_err());
    }
    assert!(
        journal(vec![allocated(1), allocated(1)])
            .validate()
            .is_err()
    );
    assert!(
        journal(vec![allocated(2), allocated(1)])
            .validate()
            .is_err()
    );
    let mut wrong = submitted(1, EditorClosureV0::None);
    wrong.binding.as_mut().unwrap().allocation.actor.owner_id += 1;
    assert!(journal(vec![wrong]).validate().is_err());
    let mut future = submitted(1, EditorClosureV0::None);
    future.submitted_service_epoch = Some(2);
    assert!(journal(vec![future]).validate().is_err());
    let mut future = permitted(1, EditorClosureV0::None);
    future.permit.as_mut().unwrap().service_epoch = 2;
    assert!(journal(vec![future]).validate().is_err());
    let mut unreserved_writer = permitted(1, EditorClosureV0::None);
    unreserved_writer.permit.as_mut().unwrap().writer_id = 17;
    assert!(journal(vec![unreserved_writer]).validate().is_err());
}
