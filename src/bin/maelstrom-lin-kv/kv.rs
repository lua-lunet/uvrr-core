//! The replicated lin-kv service. Deterministic: every replica executing the
//! same committed entry produces byte-identical output, which is what lets the
//! VRR log be the only source of ordering.
//!
//! This is the harness's stand-in for `vrr::locks::Service`. The lock service
//! cannot express lin-kv — its SET is acquire-or-renew, so it can neither
//! overwrite another holder's value nor compare-and-set from one value to a
//! different one. `Replica` treats payloads as opaque, so swapping the service
//! needs no change to the replication core.

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

#[cfg(test)]
mod tests {
    use super::*;

    fn read(kv: &mut Kv, key: i64) -> KvResponse {
        kv.execute(&KvRequest::Read {
            client_id: 1,
            request_num: 1,
            key: key.into(),
        })
    }

    #[test]
    fn read_of_absent_key_reports_key_does_not_exist() {
        let mut kv = Kv::default();
        assert!(matches!(
            read(&mut kv, 7),
            KvResponse::Failed {
                code: error::KEY_DOES_NOT_EXIST,
                ..
            }
        ));
    }

    #[test]
    fn write_then_read_returns_the_written_value() {
        let mut kv = Kv::default();
        kv.execute(&KvRequest::Write {
            client_id: 1,
            request_num: 1,
            key: 7.into(),
            value: 42.into(),
        });
        assert!(matches!(
            read(&mut kv, 7),
            KvResponse::ReadOk { ref value, .. } if value == &Value::from(42)
        ));
    }

    #[test]
    fn cas_swaps_only_on_an_exact_match_and_is_otherwise_inert() {
        let mut kv = Kv::default();
        kv.execute(&KvRequest::Write {
            client_id: 1,
            request_num: 1,
            key: 7.into(),
            value: 1.into(),
        });
        let mismatch = kv.execute(&KvRequest::Cas {
            client_id: 1,
            request_num: 2,
            key: 7.into(),
            from: 9.into(),
            to: 5.into(),
        });
        assert!(matches!(
            mismatch,
            KvResponse::Failed {
                code: error::PRECONDITION_FAILED,
                ..
            }
        ));
        assert!(
            matches!(read(&mut kv, 7), KvResponse::ReadOk { ref value, .. } if value == &Value::from(1)),
            "a failed cas must not mutate the key"
        );

        let hit = kv.execute(&KvRequest::Cas {
            client_id: 1,
            request_num: 3,
            key: 7.into(),
            from: 1.into(),
            to: 5.into(),
        });
        assert!(matches!(hit, KvResponse::CasOk { .. }));
        assert!(
            matches!(read(&mut kv, 7), KvResponse::ReadOk { ref value, .. } if value == &Value::from(5))
        );
    }

    #[test]
    fn keys_of_different_json_types_do_not_collide() {
        let mut kv = Kv::default();
        kv.execute(&KvRequest::Write {
            client_id: 1,
            request_num: 1,
            key: 7.into(),
            value: "int".into(),
        });
        kv.execute(&KvRequest::Write {
            client_id: 1,
            request_num: 2,
            key: "7".into(),
            value: "string".into(),
        });
        assert!(
            matches!(read(&mut kv, 7), KvResponse::ReadOk { ref value, .. } if value == &Value::from("int"))
        );
    }

    #[test]
    fn execution_is_deterministic_across_replicas() {
        let ops = [
            KvRequest::Write {
                client_id: 1,
                request_num: 1,
                key: 1.into(),
                value: 10.into(),
            },
            KvRequest::Cas {
                client_id: 1,
                request_num: 2,
                key: 1.into(),
                from: 10.into(),
                to: 20.into(),
            },
            KvRequest::Read {
                client_id: 1,
                request_num: 3,
                key: 1.into(),
            },
        ];
        let mut first = Kv::default();
        let mut second = Kv::default();
        for op in &ops {
            let a = serde_json::to_vec(&first.execute(op)).unwrap();
            let b = serde_json::to_vec(&second.execute(op)).unwrap();
            assert_eq!(a, b, "replicas diverged executing {op:?}");
        }
    }
}
