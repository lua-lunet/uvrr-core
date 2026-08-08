//! Maelstrom wire envelope, our peer-message carrier, and the lin-kv service
//! request/response payloads that travel opaquely through the VRR log.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One Maelstrom message. Bodies vary widely by type, so the body stays a
/// `Value` and each handler pulls the fields it needs.
#[derive(Debug, Clone, Deserialize)]
pub struct Incoming {
    pub src: String,
    #[allow(dead_code)]
    pub dest: String,
    pub body: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct Outgoing {
    pub src: String,
    pub dest: String,
    pub body: Value,
}

impl Incoming {
    pub fn kind(&self) -> &str {
        self.body.get("type").and_then(Value::as_str).unwrap_or("")
    }
    pub fn msg_id(&self) -> Option<u64> {
        self.body.get("msg_id").and_then(Value::as_u64)
    }
    pub fn field(&self, name: &str) -> Option<&Value> {
        self.body.get(name)
    }
}

/// Maelstrom error codes we use. 11 and 20/22 are the codes the `lin-kv`
/// workload understands; 11 is a *definite* failure ("this did not happen"),
/// which we may only claim when the core provably refused to append.
pub mod error {
    pub const TEMPORARILY_UNAVAILABLE: u32 = 11;
    pub const KEY_DOES_NOT_EXIST: u32 = 20;
    pub const PRECONDITION_FAILED: u32 = 22;
}

/// The lin-kv operation, as replicated. `client_id` and `request_num` are
/// carried inside the payload so a `Output::Reply` — which is only bytes —
/// can be correlated back to the Maelstrom client that is waiting.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum KvRequest {
    Read {
        client_id: u64,
        request_num: u64,
        key: Value,
    },
    Write {
        client_id: u64,
        request_num: u64,
        key: Value,
        value: Value,
    },
    Cas {
        client_id: u64,
        request_num: u64,
        key: Value,
        from: Value,
        to: Value,
    },
}

impl KvRequest {
    pub fn ids(&self) -> (u64, u64) {
        match self {
            Self::Read {
                client_id,
                request_num,
                ..
            }
            | Self::Write {
                client_id,
                request_num,
                ..
            }
            | Self::Cas {
                client_id,
                request_num,
                ..
            } => (*client_id, *request_num),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum KvResponse {
    ReadOk {
        client_id: u64,
        request_num: u64,
        value: Value,
    },
    WriteOk {
        client_id: u64,
        request_num: u64,
    },
    CasOk {
        client_id: u64,
        request_num: u64,
    },
    Failed {
        client_id: u64,
        request_num: u64,
        code: u32,
        text: String,
    },
}

impl KvResponse {
    pub fn ids(&self) -> (u64, u64) {
        match self {
            Self::ReadOk {
                client_id,
                request_num,
                ..
            }
            | Self::WriteOk {
                client_id,
                request_num,
            }
            | Self::CasOk {
                client_id,
                request_num,
            }
            | Self::Failed {
                client_id,
                request_num,
                ..
            } => (*client_id, *request_num),
        }
    }

    /// Projects the replicated result onto the Maelstrom reply body.
    pub fn body(&self) -> Value {
        match self {
            Self::ReadOk { value, .. } => {
                serde_json::json!({ "type": "read_ok", "value": value })
            }
            Self::WriteOk { .. } => serde_json::json!({ "type": "write_ok" }),
            Self::CasOk { .. } => serde_json::json!({ "type": "cas_ok" }),
            Self::Failed { code, text, .. } => {
                serde_json::json!({ "type": "error", "code": code, "text": text })
            }
        }
    }
}

/// Hex, so a VRR datagram survives Maelstrom's JSON transport while still
/// going through the real `Message::encode`/`decode` codec — including the
/// 16-byte binary header. Avoids adding a base64 dependency to the harness.
pub fn to_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(DIGITS[(byte >> 4) as usize] as char);
        out.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    out
}

pub fn from_hex(text: &str) -> Option<Vec<u8>> {
    // `%` rather than `is_multiple_of`, which is only stable since 1.87 and
    // would raise this crate's MSRV for the sake of a parity check.
    if text.len() % 2 != 0 {
        return None;
    }
    let digits = text.as_bytes();
    (0..digits.len() / 2)
        .map(|index| {
            let hi = (digits[index * 2] as char).to_digit(16)?;
            let lo = (digits[index * 2 + 1] as char).to_digit(16)?;
            Some((hi * 16 + lo) as u8)
        })
        .collect()
}
