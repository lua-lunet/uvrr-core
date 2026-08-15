//! The replicated lin-kv service. Deterministic: every replica executing the
//! same committed entry produces byte-identical output, which is what lets the
//! VRR log be the only source of ordering.
//!
//! Replay-safe under the core's at-least-once application boundary (§11): a
//! node that restores a committed history re-executes it, and re-executing a
//! prefix of these operations converges to the same state — a write sets the
//! same value again, a repeated compare-and-set fails its precondition without
//! mutating, a read is inert. `Replica` treats payloads as opaque, so the
//! service swaps in with no change to the replication core.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::proto::{KvRequest, KvResponse, error};

#[derive(Default)]
pub struct Kv {
    /// Keyed by the key's canonical JSON text, since `serde_json::Value` is
    /// not `Ord`. The original `Value` is echoed back from the request.
    entries: BTreeMap<String, Value>,
}

impl Kv {
    pub fn execute(&mut self, request: &KvRequest) -> KvResponse {
        let (client_id, request_num) = request.ids();
        match request {
            KvRequest::Read { key, .. } => match self.entries.get(&Self::slot(key)) {
                Some(value) => KvResponse::ReadOk {
                    client_id,
                    request_num,
                    value: value.clone(),
                },
                None => KvResponse::Failed {
                    client_id,
                    request_num,
                    code: error::KEY_DOES_NOT_EXIST,
                    text: "key does not exist".into(),
                },
            },
            KvRequest::Write { key, value, .. } => {
                self.entries.insert(Self::slot(key), value.clone());
                KvResponse::WriteOk {
                    client_id,
                    request_num,
                }
            }
            KvRequest::Cas { key, from, to, .. } => {
                let slot = Self::slot(key);
                match self.entries.get(&slot) {
                    None => KvResponse::Failed {
                        client_id,
                        request_num,
                        code: error::KEY_DOES_NOT_EXIST,
                        text: "key does not exist".into(),
                    },
                    Some(current) if current == from => {
                        self.entries.insert(slot, to.clone());
                        KvResponse::CasOk {
                            client_id,
                            request_num,
                        }
                    }
                    Some(current) => KvResponse::Failed {
                        client_id,
                        request_num,
                        code: error::PRECONDITION_FAILED,
                        text: format!("expected {from}, but had {current}"),
                    },
                }
            }
        }
    }

    fn slot(key: &Value) -> String {
        key.to_string()
    }
}
