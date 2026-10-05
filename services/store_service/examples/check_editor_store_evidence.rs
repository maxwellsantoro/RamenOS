//! Read-only stdin consumer of the existing pure editor-save schema.
//!
//! Exactly one CLI argument is accepted: `journal`, `receipt`, or `text`.
//! Input must reach EOF within the kind's byte bound. This example never opens
//! a path, imports a Store handler, or constructs a grant, host, or IO owner.
//!
//! Success is one newline-terminated JSON object, at most 4096 bytes including
//! the newline, with exactly `{schema_version:1, kind, byte_len, sha256, summary}`.
//! `kind` is the requested literal; `byte_len` is the full input length;
//! `sha256` and summary hashes are lowercase 64-character SHA256 hex strings.
//! Summary keys (all integers are native unsigned JSON integers) are:
//! - journal: owner_id, selected_object_id, selected_generation, revision,
//!   service_epoch, operation_high_water, writer_high_water, transition_sequence,
//!   operation_count, last_operation_id (integer or null), content_hash.
//! - receipt: outcome (`committed` or `definitive_noncommit`), owner_id,
//!   session_id, session_generation, original_instance_id,
//!   original_instance_generation, selected_object_id, selected_generation,
//!   operation_id, expected_revision, result_revision, backend_service_epoch,
//!   committed_at_ms, expected_content_hash, source_content_hash.
//! - text: selected_object_id, revision, body_byte_len, content_hash.
//!
//! A diagnostic establishes only successful pure decoding of these bytes.
//! It establishes no authority, durability, filesystem or binary provenance.
//! Invalid arguments/input emit no diagnostic; errors use a fixed stderr line
//! shorter than 256 bytes and a nonzero exit status.

use artifact_store_schema::editor_save::{
    EditorReceiptOutcomeV0, EditorSaveReceiptV0, EditorSelectionJournalV0, EditorTextHeaderV0,
    MAX_JOURNAL_BYTES, MAX_TEXT_LEN, RECEIPT_LEN, TEXT_HEADER_LEN,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io::{self, Read, Write};
use std::process::ExitCode;

const MAX_DIAGNOSTIC_BYTES: usize = 4096;

#[derive(Clone, Copy)]
enum Kind {
    Journal,
    Receipt,
    Text,
}

impl Kind {
    fn parse() -> Result<Self, Failure> {
        let mut args = std::env::args_os().skip(1);
        let argument = args.next().ok_or(Failure::Arguments)?;
        if args.next().is_some() {
            return Err(Failure::Arguments);
        }
        match argument.to_str() {
            Some("journal") => Ok(Self::Journal),
            Some("receipt") => Ok(Self::Receipt),
            Some("text") => Ok(Self::Text),
            _ => Err(Failure::Arguments),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Journal => "journal",
            Self::Receipt => "receipt",
            Self::Text => "text",
        }
    }

    fn cap(self) -> usize {
        match self {
            Self::Journal => MAX_JOURNAL_BYTES,
            Self::Receipt => RECEIPT_LEN,
            Self::Text => TEXT_HEADER_LEN + MAX_TEXT_LEN,
        }
    }
}

enum Failure {
    Arguments,
    Input,
    Decode,
    Output,
}

impl Failure {
    fn message(&self) -> &'static [u8] {
        match self {
            Self::Arguments => b"editor evidence: expected exactly journal, receipt, or text\n",
            Self::Input => b"editor evidence: unreadable, empty, or oversized stdin\n",
            Self::Decode => b"editor evidence: invalid or noncanonical payload\n",
            Self::Output => b"editor evidence: diagnostic output failed\n",
        }
    }
}

#[derive(Serialize)]
struct Diagnostic {
    schema_version: u32,
    kind: &'static str,
    byte_len: usize,
    sha256: String,
    summary: Summary,
}

#[derive(Serialize)]
#[serde(untagged)]
enum Summary {
    Journal(JournalSummary),
    Receipt(ReceiptSummary),
    Text(TextSummary),
}

#[derive(Serialize)]
struct JournalSummary {
    owner_id: u64,
    selected_object_id: u64,
    selected_generation: u64,
    revision: u64,
    service_epoch: u64,
    operation_high_water: u64,
    writer_high_water: u64,
    transition_sequence: u64,
    operation_count: usize,
    last_operation_id: Option<u64>,
    content_hash: String,
}

#[derive(Serialize)]
struct ReceiptSummary {
    outcome: EditorReceiptOutcomeV0,
    owner_id: u64,
    session_id: u64,
    session_generation: u64,
    original_instance_id: u64,
    original_instance_generation: u64,
    selected_object_id: u64,
    selected_generation: u64,
    operation_id: u64,
    expected_revision: u64,
    result_revision: u64,
    backend_service_epoch: u64,
    committed_at_ms: u64,
    expected_content_hash: String,
    source_content_hash: String,
}

#[derive(Serialize)]
struct TextSummary {
    selected_object_id: u64,
    revision: u64,
    body_byte_len: u32,
    content_hash: String,
}

fn read_bounded(input: &mut impl Read, cap: usize) -> Result<Vec<u8>, Failure> {
    let mut bytes = Vec::with_capacity(cap);
    let mut chunk = [0u8; 8192];
    while bytes.len() < cap {
        let available = chunk.len().min(cap - bytes.len());
        let count = input
            .read(&mut chunk[..available])
            .map_err(|_| Failure::Input)?;
        if count == 0 {
            return if bytes.is_empty() {
                Err(Failure::Input)
            } else {
                Ok(bytes)
            };
        }
        // The read slice bounds count before the input collection can grow.
        bytes.extend_from_slice(&chunk[..count]);
    }
    // Require EOF even for an exactly full payload; never retain a cap+1 byte.
    let mut extra = [0u8; 1];
    if input.read(&mut extra).map_err(|_| Failure::Input)? != 0 {
        return Err(Failure::Input);
    }
    Ok(bytes)
}

fn decode(kind: Kind, bytes: &[u8]) -> Result<Summary, Failure> {
    match kind {
        Kind::Journal => {
            let journal =
                EditorSelectionJournalV0::decode_canonical(bytes).map_err(|_| Failure::Decode)?;
            Ok(Summary::Journal(JournalSummary {
                owner_id: journal.owner_id,
                selected_object_id: journal.selected_object_id,
                selected_generation: journal.selected_generation,
                revision: journal.current.revision,
                service_epoch: journal.service_epoch,
                operation_high_water: journal.operation_high_water,
                writer_high_water: journal.writer_high_water,
                transition_sequence: journal.transition_sequence,
                operation_count: journal.operations.len(),
                last_operation_id: journal
                    .operations
                    .as_slice()
                    .last()
                    .map(|operation| operation.allocation.operation_id),
                content_hash: hex::encode(journal.current.content_hash),
            }))
        }
        Kind::Receipt => {
            let receipt = EditorSaveReceiptV0::decode_le(bytes).map_err(|_| Failure::Decode)?;
            if receipt.encode_le().map_err(|_| Failure::Decode)?.as_slice() != bytes {
                return Err(Failure::Decode);
            }
            Ok(Summary::Receipt(ReceiptSummary {
                outcome: receipt.outcome,
                owner_id: receipt.owner_id,
                session_id: receipt.session_id,
                session_generation: receipt.session_generation,
                original_instance_id: receipt.original_instance_id,
                original_instance_generation: receipt.original_instance_generation,
                selected_object_id: receipt.selected_object_id,
                selected_generation: receipt.selected_generation,
                operation_id: receipt.operation_id,
                expected_revision: receipt.expected_revision,
                result_revision: receipt.result_revision,
                backend_service_epoch: receipt.backend_service_epoch,
                committed_at_ms: receipt.committed_at_ms,
                expected_content_hash: hex::encode(receipt.expected_content_hash),
                source_content_hash: hex::encode(receipt.source_content_hash),
            }))
        }
        Kind::Text => {
            let header_bytes = bytes.get(..TEXT_HEADER_LEN).ok_or(Failure::Decode)?;
            let header =
                EditorTextHeaderV0::decode_le(header_bytes).map_err(|_| Failure::Decode)?;
            header
                .validate_bytes(&bytes[TEXT_HEADER_LEN..])
                .map_err(|_| Failure::Decode)?;
            if header.encode_le().map_err(|_| Failure::Decode)?.as_slice() != header_bytes {
                return Err(Failure::Decode);
            }
            Ok(Summary::Text(TextSummary {
                selected_object_id: header.selected_object_id,
                revision: header.revision,
                body_byte_len: header.byte_len,
                content_hash: hex::encode(header.content_hash),
            }))
        }
    }
}

fn run() -> Result<(), Failure> {
    let kind = Kind::parse()?;
    let bytes = read_bounded(&mut io::stdin().lock(), kind.cap())?;
    let summary = decode(kind, &bytes)?;
    let diagnostic = Diagnostic {
        schema_version: 1,
        kind: kind.name(),
        byte_len: bytes.len(),
        sha256: hex::encode(Sha256::digest(&bytes)),
        summary,
    };
    // Only fixed-size typed summaries are serialized; no input body is copied
    // into this diagnostic. Admit the complete result before writing stdout.
    let mut output = serde_json::to_vec(&diagnostic).map_err(|_| Failure::Output)?;
    if output.len() >= MAX_DIAGNOSTIC_BYTES {
        return Err(Failure::Output);
    }
    output.push(b'\n');
    io::stdout()
        .lock()
        .write_all(&output)
        .map_err(|_| Failure::Output)
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let _ = io::stderr().lock().write_all(error.message());
            ExitCode::FAILURE
        }
    }
}
