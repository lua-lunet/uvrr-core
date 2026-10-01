//! The conformance host: the abstract host interface of
//! `docs/uvrr-host-compliance.md` §2, served over the transport of §8.
//!
//! The host owns the routing and nothing else. Every replay goes through
//! [`Executor::run_case`] and [`Executor::capture`], the same execution
//! path the in-process runner and the exporter use, so what the transport
//! serves is the corpus's own reference behaviour and not a second
//! opinion of it.
//!
//! # The case endpoint carries the expectation
//!
//! `POST /case` takes the case verbatim, its `expect` included, and
//! compares the host's own capture against the expectation the request
//! carries. The host holds no corpus of its own: a client that never
//! stored the corpus still gets a verdict, and a second implementation's
//! host compares its capture against the same committed bytes without
//! carrying a corpus to compare against.

use crate::conformance::corpus::{Case, Executor, Expect, Op, assert_expectation};
use crate::conformance::server::{Request, Response};

use std::collections::BTreeMap;

/// One host: the routing, and the sessions begun through it. A `BTreeMap`
/// so the session set is ordered by id and a trace of two hosts given the
/// same requests reads the same.
pub struct Host {
    sessions: BTreeMap<String, Executor>,
    next: u64,
}

impl Default for Host {
    fn default() -> Self {
        Self::new()
    }
}

impl Host {
    /// An empty host: no sessions, the session counter at zero.
    pub fn new() -> Host {
        Host {
            sessions: BTreeMap::new(),
            next: 0,
        }
    }

    /// Routes one request. The route table is the path split into
    /// segments, matched against the method, so an endpoint's shape is
    /// legible in one match rather than spread over string tests.
    pub fn route(&mut self, request: Request) -> Response {
        let segments: Vec<&str> = request
            .path
            .trim_matches('/')
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect();
        match (request.method.as_str(), segments.as_slice()) {
            ("GET", ["health"]) => Response::json(
                200,
                serde_json::json!({
                    "host": "uvrr-conformance",
                    "crate": "uvrr-core",
                    "version": env!("CARGO_PKG_VERSION"),
                }),
            ),
            ("POST", ["case"]) => self.replay(&request.body),
            ("POST", ["session"]) => self.open(&request.body),
            ("POST", ["session", id, "op"]) => self.apply(id, &request.body),
            ("GET", ["session", id, "capture"]) => self.capture(id),
            ("GET", []) | ("GET", ["case"]) => {
                Response::reason(405, "error", "the endpoint takes POST")
            }
            // A known path with the wrong method is a method refusal, not
            // an unknown endpoint: the client can act on the first and not
            // on the second.
            ("POST", ["session", _, "capture"]) | ("POST", ["health"]) => {
                Response::reason(405, "error", "the endpoint takes GET")
            }
            _ => Response::reason(404, "error", "no such endpoint"),
        }
    }

    /// `POST /case`: replay one case and answer its verdict and its
    /// capture. The verdict is `pass` or `fail` and nothing else: a case
    /// whose setup refuses has not passed, and the reason is the
    /// mismatch, so a client reads one field whichever way it went wrong.
    fn replay(&mut self, body: &str) -> Response {
        let case: Case = match serde_json::from_str(body) {
            Ok(case) => case,
            Err(error) => {
                return Response::reason(
                    400,
                    "error",
                    &format!("the case does not parse: {error}"),
                );
            }
        };
        let (verdict, mismatch, captured) = match Executor::run_case(&case) {
            Ok(captured) => match assert_expectation(&case, &captured) {
                Ok(()) => ("pass", None, Some(captured)),
                Err(mismatch) => ("fail", Some(mismatch), Some(captured)),
            },
            Err(refusal) => ("fail", Some(refusal), None),
        };
        let mut response = serde_json::json!({
            "id": case.id,
            "family": case.family,
            "verdict": verdict,
        });
        if let Some(mismatch) = mismatch {
            response["mismatch"] = serde_json::json!(mismatch);
        }
        response["expect"] = match captured {
            Some(captured) => serde_json::to_value(&captured).unwrap_or(serde_json::Value::Null),
            None => serde_json::Value::Null,
        };
        Response::json(200, response)
    }

    /// `POST /session`: the `provision` operation's own arguments, since
    /// a session is a cluster and provisioning is what begins one.
    fn open(&mut self, body: &str) -> Response {
        let provision: Op = match serde_json::from_str(body) {
            Ok(op) => op,
            Err(error) => {
                return Response::reason(
                    400,
                    "error",
                    &format!("the body does not parse: {error}"),
                );
            }
        };
        let Op::Provision { nodes, timeout } = provision else {
            return Response::reason(400, "error", "a session opens with provision");
        };
        self.next += 1;
        let id = format!("s{}", self.next);
        self.sessions
            .insert(id.clone(), Executor::provision(nodes, timeout));
        Response::json(200, serde_json::json!({ "session": id }))
    }

    /// `POST /session/<id>/op`: one abstract host operation, applied.
    fn apply(&mut self, id: &str, body: &str) -> Response {
        let op: Op = match serde_json::from_str(body) {
            Ok(op) => op,
            Err(error) => {
                return Response::reason(
                    400,
                    "error",
                    &format!("the operation does not parse: {error}"),
                );
            }
        };
        let Some(session) = self.sessions.get_mut(id) else {
            return Response::reason(404, "error", "no such session");
        };
        match session.apply(&op) {
            Ok(()) => Response::json(200, serde_json::json!({ "ok": true })),
            Err(refusal) => {
                Response::json(200, serde_json::json!({ "ok": false, "error": refusal }))
            }
        }
    }

    /// `GET /session/<id>/capture`: the settling window, drained to
    /// quiet, as the full capture and no verdict — the caller holds the
    /// expectation and compares it itself.
    fn capture(&mut self, id: &str) -> Response {
        let Some(session) = self.sessions.get_mut(id) else {
            return Response::reason(404, "error", "no such session");
        };
        let captured: Expect = session.capture();
        Response::json(
            200,
            serde_json::json!({
                "expect": serde_json::to_value(&captured).unwrap_or(serde_json::Value::Null),
            }),
        )
    }
}
