//! Versioned JSON contract for both typed arms. No backend identity or policy lives here.
use artifact_store_schema::ContentId;
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};
use serde_json::{Value, json};
pub const MAX_REQUEST_BYTES: usize = 131_072;
pub const MAX_RESPONSE_BYTES: usize = 131_072;
pub const MAX_BYTES: usize = 65_536;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decimal(pub u64);
impl Serialize for Decimal {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0.to_string())
    }
}
impl<'de> Deserialize<'de> for Decimal {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        let n = s
            .parse::<u64>()
            .map_err(|_| D::Error::custom("u64 decimal string"))?;
        if s != n.to_string() {
            return Err(D::Error::custom("canonical decimal string"));
        }
        Ok(Self(n))
    }
}
fn encoded<'de, D: Deserializer<'de>>(d: D, prefix: &str) -> Result<u64, D::Error> {
    let s = String::deserialize(d)?;
    let hex = s
        .strip_prefix(prefix)
        .ok_or_else(|| D::Error::custom("object encoding"))?;
    if hex.len() != 16
        || !hex
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(D::Error::custom("canonical object encoding"));
    }
    let n = u64::from_str_radix(hex, 16).map_err(D::Error::custom)?;
    if n == 0 {
        return Err(D::Error::custom("zero object"));
    }
    Ok(n)
}
macro_rules! opaque {
    ($name:ident,$prefix:literal) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct $name(pub u64);
        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_str(&format!(concat!($prefix, "{:016x}"), self.0))
            }
        }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                encoded(d, $prefix).map(Self)
            }
        }
    };
}
opaque!(Cap, "cap:");
opaque!(Resource, "resource:");
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Hash(String);
impl Hash {
    pub fn new(s: String) -> Result<Self, &'static str> {
        ContentId::parse(&s).map_err(|_| "content id")?;
        Ok(Self(s))
    }
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(format!("sha256:{}", hex::encode(bytes)))
    }
    pub fn native(&self) -> [u8; 32] {
        let mut b = [0; 32];
        hex::decode_to_slice(&self.0[7..], &mut b).expect("validated hash");
        b
    }
}
impl<'de> Deserialize<'de> for Hash {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Self::new(String::deserialize(d)?).map_err(D::Error::custom)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bytes(Vec<u8>);
impl Bytes {
    pub fn new(b: Vec<u8>) -> Result<Self, &'static str> {
        if b.len() > MAX_BYTES {
            Err("byte bound")
        } else {
            Ok(Self(b))
        }
    }
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}
impl Serialize for Bytes {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&STANDARD.encode(&self.0))
    }
}
impl<'de> Deserialize<'de> for Bytes {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        if s.len() > 87_384 {
            return Err(D::Error::custom("base64 bound"));
        }
        let b = STANDARD
            .decode(&s)
            .map_err(|_| D::Error::custom("base64"))?;
        if STANDARD.encode(&b) != s {
            return Err(D::Error::custom("canonical base64"));
        }
        Self::new(b).map_err(D::Error::custom)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Right {
    Read,
    Stage,
    Validate,
    Commit,
    Observe,
}
impl Right {
    pub fn bit(self) -> u32 {
        match self {
            Self::Read => 1,
            Self::Stage => 2,
            Self::Validate => 4,
            Self::Commit => 8,
            Self::Observe => 16,
        }
    }
}
pub fn rights(bits: u32) -> Result<Vec<Right>, &'static str> {
    if bits & !31 != 0 {
        return Err("rights bits");
    }
    Ok([
        Right::Read,
        Right::Stage,
        Right::Validate,
        Right::Commit,
        Right::Observe,
    ]
    .into_iter()
    .filter(|r| bits & r.bit() != 0)
    .collect())
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum Call {
    ReadInput {
        task_cap: Cap,
        resource: Resource,
    },
    RequestGrant {
        policy_cap: Cap,
        task_id: Decimal,
        resource: Resource,
        rights: Vec<Right>,
        lifetime_ms: u32,
    },
    StageCandidate {
        task_cap: Cap,
        bytes_base64: Bytes,
    },
    ValidateCandidate {
        task_cap: Cap,
        candidate_cap: Cap,
        validator_id: Hash,
    },
    CommitCandidate {
        task_cap: Cap,
        candidate_cap: Cap,
        expected_revision: Decimal,
        expected_content_id: Hash,
    },
    GetReceipt {
        task_cap: Cap,
        commit_request_id: Decimal,
    },
    GetTaskState {
        task_cap: Cap,
    },
    RevokeGrant {
        policy_cap: Cap,
        task_cap: Cap,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema_version: u32,
    pub request_id: Decimal,
    pub call: Call,
}
pub fn decode_request(b: &[u8]) -> Result<Request, &'static str> {
    if b.len() > MAX_REQUEST_BYTES {
        return Err("request bound");
    }
    let req: Request = serde_json::from_slice(b).map_err(|_| "request syntax")?;
    if req.schema_version != 1 || req.request_id.0 == 0 {
        return Err("request header");
    }
    match &req.call {
        Call::RequestGrant {
            task_id,
            rights,
            lifetime_ms,
            ..
        } => {
            if task_id.0 == 0
                || *lifetime_ms == 0
                || *lifetime_ms > 300_000
                || rights.is_empty()
                || rights.len() > 5
            {
                return Err("grant bounds");
            }
            let mut bits = 0;
            for r in rights {
                if bits & r.bit() != 0 {
                    return Err("duplicate rights");
                }
                bits |= r.bit();
            }
        }
        Call::StageCandidate { bytes_base64, .. } if bytes_base64.0.is_empty() => {
            return Err("empty candidate");
        }
        Call::GetReceipt {
            commit_request_id, ..
        } if commit_request_id.0 == 0 => return Err("receipt id"),
        _ => {}
    }
    Ok(req)
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Ok,
    Denied,
    Invalid,
    Conflict,
    ValidationFailed,
    Expired,
    Timeout,
    Capacity,
    Io,
    RequestReuse,
    NotFound,
}
impl Status {
    pub fn from_native(n: u32) -> Option<Self> {
        Some(match n {
            0 => Self::Ok,
            1 => Self::Denied,
            2 => Self::Invalid,
            3 => Self::Conflict,
            4 => Self::ValidationFailed,
            5 => Self::Expired,
            6 => Self::Timeout,
            7 => Self::Capacity,
            8 => Self::Io,
            9 => Self::RequestReuse,
            10 => Self::NotFound,
            _ => return None,
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    NotRun,
    Valid,
    Invalid,
    Timeout,
    HostFailure,
}
impl Outcome {
    pub fn from_native(n: u32) -> Option<Self> {
        Some(match n {
            0 => Self::NotRun,
            1 => Self::Valid,
            2 => Self::Invalid,
            3 => Self::Timeout,
            4 => Self::HostFailure,
            _ => return None,
        })
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelValidation {
    pub candidate_id: Hash,
    pub schema_id: Hash,
    pub policy_id: Hash,
    pub validator_id: Hash,
    pub generation: Decimal,
    pub valid_until_ms: Decimal,
    pub wall_elapsed_ms: Decimal,
    pub guest_elapsed_ms: Decimal,
    pub diagnostics_bytes: Decimal,
    pub diagnostics_truncated: bool,
    pub outcome: Outcome,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelState {
    pub task_id: Decimal,
    pub resource: Resource,
    pub revision: Decimal,
    pub content_id: Hash,
    pub generation: Decimal,
    pub rights: Vec<Right>,
    pub expires_at_ms: Decimal,
    pub now_ms: Decimal,
    pub schema_id: Hash,
    pub policy_id: Hash,
    pub validator_id: Hash,
    pub input_resources: [Resource; 3],
    pub validation: Option<ModelValidation>,
    pub validation_current: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum Reply {
    ReadInput {
        content_id: Hash,
        bytes_base64: Bytes,
    },
    RequestGrant {
        task_cap: Cap,
        generation: Decimal,
        expires_at_ms: Decimal,
        rights: Vec<Right>,
    },
    StageCandidate {
        candidate_cap: Cap,
        content_id: Hash,
    },
    ValidateCandidate {
        outcome: Outcome,
        valid_until_ms: Decimal,
        diagnostics_base64: Bytes,
        truncated: bool,
    },
    CommitCandidate {
        receipt_cap: Cap,
        revision: Decimal,
        content_id: Hash,
    },
    GetReceipt {
        commit_request_id: Decimal,
        revision: Decimal,
        content_id: Hash,
    },
    GetTaskState {
        state: Box<ModelState>,
    },
    RevokeGrant {
        generation: Decimal,
        revoked_count: u32,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Response {
    pub schema_version: u32,
    pub request_id: Option<Decimal>,
    pub status: Status,
    pub result: Option<Reply>,
}
impl Response {
    pub fn error(request_id: Option<Decimal>, status: Status) -> Self {
        Self {
            schema_version: 1,
            request_id,
            status,
            result: None,
        }
    }
}
pub fn encode_response(r: &Response) -> Result<Vec<u8>, &'static str> {
    if r.schema_version != 1
        || matches!(r.status, Status::Denied | Status::Expired) && r.result.is_some()
        || r.status == Status::Ok && (r.request_id.is_none() || r.result.is_none())
    {
        return Err("response redaction");
    }
    let b = serde_json::to_vec(r).map_err(|_| "response syntax")?;
    if b.len() > MAX_RESPONSE_BYTES {
        Err("response bound")
    } else {
        Ok(b)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceBinding {
    pub resource: Resource,
    pub logical_resource: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bootstrap {
    pub schema_version: u32,
    pub task_id: Decimal,
    pub policy_cap: Cap,
    pub resources: [ResourceBinding; 3],
}
fn decimal_schema(positive: bool) -> Value {
    // Exact upper bound as a string, including JS-safe transport of u64::MAX.
    let max = "18446744073709551615";
    let mut alternatives = vec!["[1-9][0-9]{0,18}".to_owned()];
    if !positive {
        alternatives.push("0".into());
    }
    for (i, c) in max.bytes().enumerate() {
        let low = if i == 0 { b'1' } else { b'0' };
        if c > low {
            let range = if c == low + 1 {
                (low as char).to_string()
            } else {
                format!("[{}-{}]", low as char, (c - 1) as char)
            };
            let tail = if i == 19 {
                String::new()
            } else {
                format!("[0-9]{{{}}}", 19 - i)
            };
            alternatives.push(format!("{}{}{}", &max[..i], range, tail));
        }
    }
    alternatives.push(max.into());
    json!({"type":"string","pattern":format!("^({})$(?![\\s\\S])",alternatives.join("|"))})
}
fn object(properties: Value) -> Value {
    let required: Vec<_> = properties
        .as_object()
        .expect("schema map")
        .keys()
        .cloned()
        .collect();
    json!({"type":"object","additionalProperties":false,"required":required,"properties":properties})
}
pub fn tool_contract() -> Vec<u8> {
    let cap =
        json!({"type":"string","pattern":"^cap:(?!0000000000000000$)[0-9a-f]{16}$(?![\\s\\S])"});
    let resource = json!({"type":"string","pattern":"^resource:(?!0000000000000000$)[0-9a-f]{16}$(?![\\s\\S])"});
    let hash = json!({"type":"string","pattern":"^sha256:[0-9a-f]{64}$(?![\\s\\S])"});
    let bytes = bytes_schema(false);
    let rights = json!({"type":"array","minItems":1,"maxItems":5,"uniqueItems":true,"items":{"enum":["read","stage","validate","commit","observe"]}});
    let specifications = [
        (
            "read_input",
            "Read one resource using the supplied current task grant.",
            json!({"task_cap":cap,"resource":resource}),
        ),
        (
            "request_grant",
            "Request rights and lifetime for this task under the supplied policy capability. The backend decides whether to grant them.",
            json!({"policy_cap":cap,"task_id":decimal_schema(true),"resource":resource,"rights":rights,"lifetime_ms":{"type":"integer","minimum":1,"maximum":300000}}),
        ),
        (
            "stage_candidate",
            "Stage exact bytes as an immutable candidate. Staging does not publish output.",
            json!({"task_cap":cap,"bytes_base64":bytes}),
        ),
        (
            "validate_candidate",
            "Run the pinned validator on an existing candidate under current validation authority.",
            json!({"task_cap":cap,"candidate_cap":cap,"validator_id":hash}),
        ),
        (
            "commit_candidate",
            "Commit a validated candidate against both prior revision and content ID. Repeating the same successful request ID and semantic fields returns its original receipt; different fields fail.",
            json!({"task_cap":cap,"candidate_cap":cap,"expected_revision":decimal_schema(false),"expected_content_id":hash}),
        ),
        (
            "get_receipt",
            "Read the original successful commit receipt using current commit authority.",
            json!({"task_cap":cap,"commit_request_id":decimal_schema(true)}),
        ),
        (
            "get_task_state",
            "Read this task's current accepted output, pins, grant and validation freshness using observation authority.",
            json!({"task_cap":cap}),
        ),
        (
            "revoke_grant",
            "Revoke this task generation using policy authority. This invalidates all current task grants; candidates and durable receipts remain locators.",
            json!({"policy_cap":cap,"task_cap":cap}),
        ),
    ];
    let tools:Vec<_>=specifications.into_iter().map(|(name,description,mut properties)|{
        properties["operation"]=json!({"const":name});
        json!({"name":name,"description":description,"input_schema":object(json!({"schema_version":{"const":1},"request_id":decimal_schema(true),"call":object(properties)}))})
    }).collect();
    serde_json::to_vec(&json!({"schema_version":1,"transport":"one JSON request/response per line","max_request_bytes":MAX_REQUEST_BYTES,"max_response_bytes":MAX_RESPONSE_BYTES,
        "status_vocabulary":["ok","denied","invalid","conflict","validation_failed","expired","timeout","capacity","io","request_reuse","not_found"],
        "tools":tools,"response_schema":response_schema(),"bootstrap_schema":bootstrap_schema()})).expect("static schema")
}

fn bytes_schema(empty: bool) -> Value {
    json!({"type":"string","minLength":if empty{0}else{4},"maxLength":87384,
      "pattern":"^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/][AQgw]==|[A-Za-z0-9+/]{2}[AEIMQUYcgkosw048]=)?$(?![\\s\\S])",
      "anyOf":[{"maxLength":87380},{"minLength":87384,"pattern":"==$(?![\\s\\S])"}],"contentEncoding":"base64"})
}
fn response_schema() -> Value {
    let cap =
        json!({"type":"string","pattern":"^cap:(?!0000000000000000$)[0-9a-f]{16}$(?![\\s\\S])"});
    let resource = json!({"type":"string","pattern":"^resource:(?!0000000000000000$)[0-9a-f]{16}$(?![\\s\\S])"});
    let hash = json!({"type":"string","pattern":"^sha256:[0-9a-f]{64}$(?![\\s\\S])"});
    let rights = json!({"type":"array","minItems":1,"maxItems":5,"uniqueItems":true,"items":{"enum":["read","stage","validate","commit","observe"]}});
    let outcome = json!({"enum":["not_run","valid","invalid","timeout","host_failure"]});
    let validation = object(
        json!({"candidate_id":hash,"schema_id":hash,"policy_id":hash,"validator_id":hash,
      "generation":decimal_schema(true),"valid_until_ms":decimal_schema(false),"wall_elapsed_ms":decimal_schema(false),
      "guest_elapsed_ms":decimal_schema(false),"diagnostics_bytes":decimal_schema(false),"diagnostics_truncated":{"type":"boolean"},"outcome":outcome}),
    );
    let state = object(
        json!({"task_id":decimal_schema(true),"resource":resource,"revision":decimal_schema(false),"content_id":hash,
      "generation":decimal_schema(true),"rights":rights,"expires_at_ms":decimal_schema(true),"now_ms":decimal_schema(false),
      "schema_id":hash,"policy_id":hash,"validator_id":hash,"input_resources":{"type":"array","minItems":3,"maxItems":3,"items":resource},
      "validation":{"anyOf":[{"type":"null"},validation]},"validation_current":{"type":"boolean"}}),
    );
    let replies = [
        (
            "read_input",
            json!({"content_id":hash,"bytes_base64":bytes_schema(false)}),
        ),
        (
            "request_grant",
            json!({"task_cap":cap,"generation":decimal_schema(true),"expires_at_ms":decimal_schema(true),"rights":rights}),
        ),
        (
            "stage_candidate",
            json!({"candidate_cap":cap,"content_id":hash}),
        ),
        (
            "validate_candidate",
            json!({"outcome":outcome,"valid_until_ms":decimal_schema(false),"diagnostics_base64":bytes_schema(true),"truncated":{"type":"boolean"}}),
        ),
        (
            "commit_candidate",
            json!({"receipt_cap":cap,"revision":decimal_schema(true),"content_id":hash}),
        ),
        (
            "get_receipt",
            json!({"commit_request_id":decimal_schema(true),"revision":decimal_schema(true),"content_id":hash}),
        ),
        ("get_task_state", json!({"state":state})),
        (
            "revoke_grant",
            json!({"generation":decimal_schema(true),"revoked_count":{"type":"integer","minimum":0,"maximum":4294967295u64}}),
        ),
    ];
    let schemas: Vec<_> = replies
        .into_iter()
        .map(|(name, mut fields)| {
            fields["operation"] = json!({"const":name});
            object(fields)
        })
        .collect();
    let mut schema = object(
        json!({"schema_version":{"const":1},"request_id":{"anyOf":[{"type":"null"},decimal_schema(true)]},
      "status":{"enum":["ok","denied","invalid","conflict","validation_failed","expired","timeout","capacity","io","request_reuse","not_found"]},
      "result":{"anyOf":[{"type":"null"},{"oneOf":schemas}]}}),
    );
    schema["allOf"] = json!([
      {"if":{"properties":{"status":{"enum":["denied","expired"]}}},"then":{"properties":{"result":{"type":"null"}}}},
      {"if":{"properties":{"status":{"const":"ok"}}},"then":{"properties":{"result":{"type":"object"},"request_id":decimal_schema(true)}}}
    ]);
    schema
}
fn bootstrap_schema() -> Value {
    object(
        json!({"schema_version":{"const":1},"task_id":decimal_schema(true),
        "policy_cap":{"type":"string","pattern":"^cap:(?!0000000000000000$)[0-9a-f]{16}$(?![\\s\\S])"},
        "resources":{"type":"array","minItems":3,"maxItems":3,"items":object(json!({
          "resource":{"type":"string","pattern":"^resource:(?!0000000000000000$)[0-9a-f]{16}$(?![\\s\\S])"},
          "logical_resource":{"type":"string","maxLength":128}
        }))}}),
    )
}
