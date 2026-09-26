//! The reconfiguration plan: the artefact the operator computes once, submits,
//! and the leader steps through while the cluster keeps running normally
//! (`docs/weighted-reconfiguration-solver.md`).
//!
//! A [`Plan`] is serde-free: it is the core's type, built from the solver's
//! schedule ([`crate::solver`]) and replayed by the leader one batch per era.
//! The JSONL form exists only at the tool perimeter, the codec below is
//! feature-gated, and a plan crossing that perimeter is validated once, on
//! the way in, against the same transition gates the leader applies
//! ([`Plan::validate_against`]). The core never sees JSON.

use crate::configuration::{ConfigError, Configuration, Member, SystemOperation};
use crate::ids::{NodeId, Slot};

#[cfg(feature = "serde")]
use crate::configuration::Snapshot;
#[cfg(feature = "serde")]
use crate::ids::Era;

/// A computed reconfiguration: the membership it starts from, and one batch of
/// operations per era, in commit order.
///
/// `initial` is the configuration the plan was computed against, membership
/// order included, because order is succession, and `steps[i]` is the batch
/// that establishes the era after step `i − 1`. The plan carries nothing the
/// ordinary protocol does not already carry: each step is committed as ONE
/// [`SystemOperation::Batch`], exactly as any other reconfiguration.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Plan {
    /// The membership the plan was computed against, in succession order.
    pub initial: Vec<Member>,
    /// One batch per era, in commit order; each batch is applied as ONE
    /// [`SystemOperation::Batch`].
    pub steps: Vec<Vec<SystemOperation>>,
}

/// Why a plan may not be stepped through.
///
/// One variant per refusal, so a host reacting to a rejection knows which
/// precondition failed: whether the plan no longer matches the committed
/// configuration it claims to start from, or a step the fold refuses.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PlanRejection {
    /// The plan's initial membership is not the committed membership: a member
    /// is missing, extra, or in a different succession position.
    InitialMembership {
        /// The plan's initial membership.
        plan: Vec<Member>,
        /// The committed membership.
        committed: Vec<Member>,
    },
    /// The ordered membership matches but a member's weight differs.
    InitialWeight {
        /// The first member whose weight differs.
        node: NodeId,
        /// The weight the plan claims.
        plan: crate::configuration::Weight,
        /// The committed weight.
        committed: crate::configuration::Weight,
    },
    /// A step is illegal against the configuration its predecessors establish:
    /// the plan was legal when computed but has drifted, or was never legal.
    Step {
        /// The zero-based index of the refused step.
        index: usize,
        /// The fold's refusal, by name.
        refusal: ConfigError,
    },
}

impl std::fmt::Display for PlanRejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InitialMembership { .. } => write!(
                f,
                "the plan's initial membership is not the committed membership \
                 (membership or succession order differs)"
            ),
            Self::InitialWeight {
                node,
                plan,
                committed,
            } => write!(
                f,
                "member {node:?} votes with {:?} in the plan but {:?} is committed",
                plan.0, committed.0
            ),
            Self::Step { index, refusal } => write!(f, "step {index} is refused: {refusal:?}"),
        }
    }
}
impl std::error::Error for PlanRejection {}

/// Folds `steps` onto `current`, one batch per era, yielding the final
/// configuration or the first refusing step. Shared by the leader-side
/// validation and the JSONL codec, so the perimeter and the leader cannot
/// disagree about what a plan reaches.
fn fold_steps(
    current: &Configuration,
    steps: &[Vec<SystemOperation>],
) -> Result<Configuration, (usize, ConfigError)> {
    let mut config = current.clone();
    for (index, ops) in steps.iter().enumerate() {
        config = config
            .apply(&SystemOperation::Batch(ops.clone()), Slot(0))
            .map_err(|refusal| (index, refusal))?;
    }
    Ok(config)
}

impl Plan {
    /// Checks the plan against the leader's current committed configuration.
    ///
    /// The plan's `initial` must equal the committed configuration,
    /// membership, succession order and weights all compared, and every step
    /// must fold through the ordinary transition gates
    /// ([`Configuration::apply`]). A plan that was legal when computed but has
    /// drifted is rejected by name, never committed.
    pub fn validate_against(&self, current: &Configuration) -> Result<(), PlanRejection> {
        let committed = current.order();
        if self.initial.len() != committed.len()
            || self
                .initial
                .iter()
                .zip(committed)
                .any(|(member, actual)| member.node != actual.node)
        {
            return Err(PlanRejection::InitialMembership {
                plan: self.initial.clone(),
                committed: committed.to_vec(),
            });
        }
        for (member, actual) in self.initial.iter().zip(committed) {
            if member.weight != actual.weight {
                return Err(PlanRejection::InitialWeight {
                    node: member.node,
                    plan: member.weight,
                    committed: actual.weight,
                });
            }
        }
        fold_steps(current, &self.steps)
            .map_err(|(index, refusal)| PlanRejection::Step { index, refusal })?;
        Ok(())
    }
}

/// Why a plan did not cross the JSON perimeter.
#[cfg(feature = "serde")]
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PlanCodecError {
    /// A line was not valid JSON of the plan schema.
    Syntax {
        /// The one-based file line that refused.
        line: usize,
        /// What the line was expected to be.
        detail: String,
    },
    /// The line sequence was not a plan: the header is missing or misplaced,
    /// the version is unsupported, or an operation is outside the alphabet.
    Shape(&'static str),
    /// The initial membership is not a legal configuration.
    Initial(ConfigError),
    /// A step does not fold from the configuration its predecessors establish.
    Step {
        /// The zero-based index of the refused step.
        index: usize,
        /// The fold's refusal, by name.
        refusal: ConfigError,
    },
    /// The declared target is not the configuration the steps reach.
    Target {
        /// The target the header declared.
        declared: Vec<Member>,
        /// The configuration the steps actually reach.
        computed: Vec<Member>,
    },
}

#[cfg(feature = "serde")]
impl std::fmt::Display for PlanCodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Syntax { line, detail } => write!(f, "line {line}: {detail}"),
            Self::Shape(shape) => write!(f, "{shape}"),
            Self::Initial(error) => {
                write!(
                    f,
                    "the initial membership is not a legal configuration: {error:?}"
                )
            }
            Self::Step { index, refusal } => write!(f, "step {index} is refused: {refusal:?}"),
            Self::Target { .. } => {
                write!(
                    f,
                    "the declared target is not the configuration the steps reach"
                )
            }
        }
    }
}
#[cfg(feature = "serde")]
impl std::error::Error for PlanCodecError {}

/// The JSONL wire shapes: the schema of `docs/weighted-reconfiguration-solver.md`,
/// one JSON object per line. These types never leak past the codec.
#[cfg(feature = "serde")]
mod jsonl {
    use crate::configuration::{Member, SystemOperation, Weight};
    use crate::ids::NodeId;
    use serde::{Deserialize, Serialize};

    /// One member as the schema names it: `{"id":N,"weight":W}`.
    #[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct JsonMember {
        /// The member's identity.
        pub id: u32,
        /// The member's voting weight.
        pub weight: u32,
    }

    /// One operation as the schema names it: `{"op":...}` with `node` and
    /// `position` present exactly where the operation carries them.
    #[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct JsonOp {
        /// The operation name: `increment`, `decrement`, `double`, `halve`,
        /// `join`, or `leave`.
        pub op: String,
        /// The node the operation names, when it names one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub node: Option<u32>,
        /// The succession position, for `join` only.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub position: Option<u32>,
    }

    /// The header line: what the plan starts from and what it reaches.
    #[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct PlanLine {
        /// Always `"plan"`.
        pub kind: String,
        /// Always `1`.
        pub version: u32,
        /// The membership the plan was computed against.
        pub initial: Vec<JsonMember>,
        /// The membership the steps reach.
        pub target: Vec<JsonMember>,
    }

    /// One step line: one era's batch.
    #[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct StepLine {
        /// Always `"step"`.
        pub kind: String,
        /// The batch's operations, in application order.
        pub ops: Vec<JsonOp>,
    }

    /// The member's core form.
    pub fn member_of(json: &JsonMember) -> Member {
        Member {
            node: NodeId(json.id),
            weight: Weight(json.weight),
        }
    }

    /// The member's wire form.
    pub fn json_of(member: &Member) -> JsonMember {
        JsonMember {
            id: member.node.0,
            weight: member.weight.0,
        }
    }

    /// The operation's wire form, or `None` for an operation the JSONL
    /// alphabet does not name (genesis and nested batches: no solver schedule
    /// contains one, and a plan that did is not a plan).
    pub fn json_of_op(op: &SystemOperation) -> Option<JsonOp> {
        Some(match op {
            SystemOperation::Increment(node) => JsonOp {
                op: "increment".to_string(),
                node: Some(node.0),
                position: None,
            },
            SystemOperation::Decrement(node) => JsonOp {
                op: "decrement".to_string(),
                node: Some(node.0),
                position: None,
            },
            SystemOperation::Double => JsonOp {
                op: "double".to_string(),
                node: None,
                position: None,
            },
            SystemOperation::Halve => JsonOp {
                op: "halve".to_string(),
                node: None,
                position: None,
            },
            SystemOperation::Join { node, position } => JsonOp {
                op: "join".to_string(),
                node: Some(node.0),
                position: Some(*position),
            },
            SystemOperation::Leave(node) => JsonOp {
                op: "leave".to_string(),
                node: Some(node.0),
                position: None,
            },
            SystemOperation::Void | SystemOperation::Init { .. } | SystemOperation::Batch(_) => {
                return None;
            }
        })
    }

    /// The operation's core form, refusing the names and field shapes the
    /// alphabet does not admit.
    pub fn op_of_json(json: &JsonOp) -> Result<SystemOperation, &'static str> {
        let node = json.node.map(NodeId);
        match json.op.as_str() {
            "increment" => match (node, json.position) {
                (Some(node), None) => Ok(SystemOperation::Increment(node)),
                _ => Err("increment names exactly one node and no position"),
            },
            "decrement" => match (node, json.position) {
                (Some(node), None) => Ok(SystemOperation::Decrement(node)),
                _ => Err("decrement names exactly one node and no position"),
            },
            "double" if node.is_none() && json.position.is_none() => Ok(SystemOperation::Double),
            "halve" if node.is_none() && json.position.is_none() => Ok(SystemOperation::Halve),
            "join" => match (node, json.position) {
                (Some(node), Some(position)) => Ok(SystemOperation::Join { node, position }),
                _ => Err("join names exactly one node and one position"),
            },
            "leave" => match (node, json.position) {
                (Some(node), None) => Ok(SystemOperation::Leave(node)),
                _ => Err("leave names exactly one node and no position"),
            },
            _ => Err("unknown operation name"),
        }
    }
}

#[cfg(feature = "serde")]
fn inflate_members(order: Vec<crate::configuration::Member>) -> Result<Configuration, ConfigError> {
    Snapshot { era: Era(1), order }.inflate()
}

#[cfg(feature = "serde")]
fn parse_line<T: serde::de::DeserializeOwned>(
    line: &str,
    number: usize,
) -> Result<T, PlanCodecError> {
    serde_json::from_str(line).map_err(|error| PlanCodecError::Syntax {
        line: number,
        detail: error.to_string(),
    })
}

#[cfg(feature = "serde")]
impl Plan {
    /// The plan's JSONL form: the header line, then one step line per era.
    ///
    /// The target is computed, not stored: it is the configuration the steps
    /// actually reach, folded through the same gates the leader applies.
    pub fn to_jsonl(&self) -> Result<String, PlanCodecError> {
        let initial = inflate_members(self.initial.clone()).map_err(PlanCodecError::Initial)?;
        let target = fold_steps(&initial, &self.steps)
            .map_err(|(index, refusal)| PlanCodecError::Step { index, refusal })?;
        let header = jsonl::PlanLine {
            kind: "plan".to_string(),
            version: 1,
            initial: self.initial.iter().map(jsonl::json_of).collect(),
            target: target.order().iter().map(jsonl::json_of).collect(),
        };
        let mut lines =
            vec![
                serde_json::to_string(&header).map_err(|error| PlanCodecError::Syntax {
                    line: 1,
                    detail: error.to_string(),
                })?,
            ];
        for ops in &self.steps {
            let mut wire = Vec::with_capacity(ops.len());
            for op in ops {
                wire.push(jsonl::json_of_op(op).ok_or(PlanCodecError::Shape(
                    "the plan carries an operation the JSONL alphabet does not name",
                ))?);
            }
            let line = jsonl::StepLine {
                kind: "step".to_string(),
                ops: wire,
            };
            lines.push(
                serde_json::to_string(&line).map_err(|error| PlanCodecError::Syntax {
                    line: lines.len() + 1,
                    detail: error.to_string(),
                })?,
            );
        }
        let mut text = lines.join("\n");
        text.push('\n');
        Ok(text)
    }

    /// Parses plan JSONL: the header line, then one step line per era.
    ///
    /// The perimeter validates once, on the way in: the initial membership
    /// must inflate through the checked snapshot constructor, every step must
    /// fold from the configuration its predecessors establish, and the
    /// declared target must be the configuration the steps actually reach.
    pub fn from_jsonl(text: &str) -> Result<Plan, PlanCodecError> {
        let mut header: Option<(Vec<Member>, Vec<Member>)> = None;
        let mut steps: Vec<Vec<SystemOperation>> = Vec::new();
        for (number, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            if header.is_none() {
                let line: jsonl::PlanLine = parse_line(line, number + 1)?;
                if line.kind != "plan" {
                    return Err(PlanCodecError::Shape(
                        "the first line is not the plan header",
                    ));
                }
                if line.version != 1 {
                    return Err(PlanCodecError::Shape("unsupported plan version"));
                }
                header = Some((
                    line.initial.iter().map(jsonl::member_of).collect(),
                    line.target.iter().map(jsonl::member_of).collect(),
                ));
            } else {
                let line: jsonl::StepLine = parse_line(line, number + 1)?;
                if line.kind != "step" {
                    return Err(PlanCodecError::Shape("a step line is not a step"));
                }
                let ops = line
                    .ops
                    .iter()
                    .map(jsonl::op_of_json)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|detail| PlanCodecError::Syntax {
                        line: number + 1,
                        detail: detail.to_string(),
                    })?;
                steps.push(ops);
            }
        }
        let Some((initial, target)) = header else {
            return Err(PlanCodecError::Shape("the plan header is missing"));
        };
        let initial_config = inflate_members(initial.clone()).map_err(PlanCodecError::Initial)?;
        let computed = fold_steps(&initial_config, &steps)
            .map_err(|(index, refusal)| PlanCodecError::Step { index, refusal })?;
        let computed: Vec<Member> = computed.order().to_vec();
        if computed != target {
            return Err(PlanCodecError::Target {
                declared: target,
                computed,
            });
        }
        Ok(Plan { initial, steps })
    }
}
