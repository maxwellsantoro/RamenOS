//! Preparation-only stdin verifier. No Store handler, path IO or grant authority.
//! The independently reviewed runner chooses one source-fixed profile mode.
use artifact_store_schema::editor_save::BoundedVec;
use artifact_store_schema::{
    Manifest,
    signature::{
        SignatureAlgorithm, SignaturePolicy, SignatureValidationConfig, SignatureValidationResult,
        TrustedKeys, manifest_signing_bytes, validate_manifest_signatures,
    },
};
use ed25519_dalek::SigningKey;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    io::{self, Read, Write},
    process::ExitCode,
};

const INPUT_CAP: usize = 16384;
const OUTPUT_CAP: usize = 1024;
const FIXTURE_SEED: [u8; 32] = [0x53; 32];
const KIND: &str = "editor-selected-private";
const CHANNEL: &str = "dev-host-editor";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    body_hex: String,
    manifest_hex: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictManifest {
    schema_version: u32,
    content_id: String,
    size_bytes: u64,
    kind: String,
    channels: BoundedVec<String, 1>,
    signatures: BoundedVec<String, 1>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictSignature {
    algorithm: SignatureAlgorithm,
    signature_data: String,
    key_id: String,
}
#[derive(Serialize)]
struct Diagnostic {
    schema_version: u32,
    kind: &'static str,
    profile_mode: &'static str,
    key_id: &'static str,
    public_key: String,
    manifest_byte_len: usize,
    manifest_sha256: String,
    body_byte_len: usize,
    body_sha256: String,
    signing_policy_sha256: String,
}

fn profile(mode: &str) -> Result<(&'static str, &'static str), ()> {
    match mode {
        "native-task" => Ok(("native-task", "native-task-v0-fixture")),
        "native-read" => Ok(("native-read", "native-read-fixture-v0")),
        _ => Err(()),
    }
}
fn hash(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
fn unhex(value: &str, cap: usize, nonempty: bool) -> Result<Vec<u8>, ()> {
    if value.len() > cap * 2
        || !value.len().is_multiple_of(2)
        || (nonempty && value.is_empty())
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(());
    }
    hex::decode(value).map_err(|_| ())
}
fn bounded_read(reader: &mut impl Read) -> Result<Vec<u8>, ()> {
    let mut bytes = Vec::with_capacity(INPUT_CAP);
    let mut chunk = [0; 4096];
    while bytes.len() < INPUT_CAP {
        let available = chunk.len().min(INPUT_CAP - bytes.len());
        let n = reader.read(&mut chunk[..available]).map_err(|_| ())?;
        if n == 0 {
            return if bytes.is_empty() { Err(()) } else { Ok(bytes) };
        }
        bytes.extend_from_slice(&chunk[..n]);
    }
    let mut extra = [0];
    if reader.read(&mut extra).map_err(|_| ())? != 0 {
        return Err(());
    }
    Ok(bytes)
}
fn verify(mode: &str, raw: &[u8]) -> Result<Diagnostic, ()> {
    let (mode, key_id) = profile(mode)?;
    if raw.is_empty() || raw.len() > INPUT_CAP {
        return Err(());
    }
    let input: Input = serde_json::from_slice(raw).map_err(|_| ())?;
    let body = unhex(&input.body_hex, 4096, false)?;
    let manifest_raw = unhex(&input.manifest_hex, 2048, true)?;
    let m: StrictManifest = serde_json::from_slice(&manifest_raw).map_err(|_| ())?;
    let body_hash = hash(&body);
    if m.schema_version != 1
        || m.content_id != format!("sha256:{body_hash}")
        || m.size_bytes != body.len() as u64
        || m.kind != KIND
        || m.channels.as_slice() != [CHANNEL]
        || m.signatures.as_slice().len() != 1
        || m.signatures.as_slice()[0].len() > 1024
    {
        return Err(());
    }
    let signature: StrictSignature =
        serde_json::from_str(&m.signatures.as_slice()[0]).map_err(|_| ())?;
    if signature.algorithm != SignatureAlgorithm::Ed25519
        || signature.key_id != key_id
        || signature.signature_data.len() > 128
    {
        return Err(());
    }
    let manifest = Manifest {
        schema_version: m.schema_version,
        content_id: m.content_id,
        size_bytes: m.size_bytes,
        kind: m.kind,
        channels: m.channels.as_slice().to_vec(),
        signatures: m.signatures.as_slice().to_vec(),
    };
    let public = SigningKey::from_bytes(&FIXTURE_SEED)
        .verifying_key()
        .to_bytes();
    let mut trusted = TrustedKeys::new();
    trusted
        .add_ed25519_key(key_id.to_owned(), &public)
        .map_err(|_| ())?;
    let policy = SignatureValidationConfig {
        policy: SignaturePolicy::RequireSignature,
        trusted_key_ids: vec![key_id.to_owned()],
        max_signature_age_secs: None,
        trusted_keys: trusted,
    };
    if validate_manifest_signatures(
        &manifest.signatures,
        &manifest_signing_bytes(&manifest).map_err(|_| ())?,
        &policy,
    ) != SignatureValidationResult::Valid
    {
        return Err(());
    }
    let policy_raw = serde_json::to_vec(&(1u32, "RequireSignature", KIND, CHANNEL, key_id, public))
        .map_err(|_| ())?;
    Ok(Diagnostic {
        schema_version: 1,
        kind: "fixture_manifest",
        profile_mode: mode,
        key_id,
        public_key: hex::encode(public),
        manifest_byte_len: manifest_raw.len(),
        manifest_sha256: hash(&manifest_raw),
        body_byte_len: body.len(),
        body_sha256: body_hash,
        signing_policy_sha256: hash(&policy_raw),
    })
}
fn execute() -> Result<(), ()> {
    let mut args = std::env::args_os().skip(1);
    let mode = args.next().ok_or(())?;
    if args.next().is_some() {
        return Err(());
    }
    let raw = bounded_read(&mut io::stdin().lock())?;
    let diagnostic = verify(mode.to_str().ok_or(())?, &raw)?;
    let mut output = serde_json::to_vec(&diagnostic).map_err(|_| ())?;
    if output.len() + 1 > OUTPUT_CAP {
        return Err(());
    }
    output.push(b'\n');
    io::stdout().lock().write_all(&output).map_err(|_| ())
}
fn main() -> ExitCode {
    match execute() {
        Ok(()) => ExitCode::SUCCESS,
        Err(()) => {
            let _ = io::stderr()
                .lock()
                .write_all(b"native manifest evidence: denied\n");
            ExitCode::FAILURE
        }
    }
}
