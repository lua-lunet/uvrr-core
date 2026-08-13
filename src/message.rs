//! The message vocabulary: [`Header`] plus the protocol bodies.
//!
//! Spec §4 (state transfer), §6 (normal operation), §9 (view change), §10
//! (recovery), §11 (client boundary), §13.3 (piggybacked commit frontier), and
//! decisions W1 (era in every header), W3 (normative exact lengths), W4
//! (fixed-width big-endian), S4 (the recovery nonce is the tick).
//!
//! A [`Message`] is the 20-byte [`Header`] followed by a one-byte body
//! discriminant and the body fields. The kind travels twice — once as the
//! header's `u32` [`Tag`], once as the body's `u8` discriminant — and the two
//! must agree: a disagreement is malformed, never guessed at, because the wire
//! cannot say which field lied. Body discriminants mirror the [`Tag`] numbering
//! exactly, and `0` is reserved for the same reason it is reserved in
//! [`wire::Tag`]: an all-zero buffer is not a message.
//!
//! The header slot field is framed for regularity, not because every message
//! names a position. What a tag's header slot may legally carry is
//! [`crate::invariant::header_slot_role`]'s table, and this module stays
//! agnostic about it exactly as [`crate::wire`] does: the codec encodes the
//! fields, the legality checker rules on them.
//!
//! [`Header`]: crate::wire::Header
//! [`wire::Tag`]: crate::wire::Tag

use crate::configuration::SystemOperation;
use crate::ids::{ClientId, RequestNumber, Slot, Tick, ViewId};
use crate::journal::LogEntry;
use crate::wire::{
    Header, Malformed, Pack, PackWriter, Tag, Unpack, UnpackCursor, UnpackError, opaque_packed_len,
};

/// One protocol datagram: the fixed header and the body it authorises.
///
/// `header.view` carries the era authorising the message as a transport-visible
/// fact (W1): under overlap mode (§8.7.7) a primary addresses era `e` and era
/// `e+1` at the same time, so the era cannot be a property the recipient must
/// derive from protocol state.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Message {
    /// Which body follows, the authorising view, and the slot field whose legal
    /// values [`crate::invariant::header_slot_role`] fixes per tag.
    pub header: Header,
    /// The message payload.
    pub body: Body,
}

/// The protocol body alphabet: VSR-2012 §4/§5 plus the era evidence this
/// design adds (§8.7.7–§8.7.8).
///
/// Every variant names its [`Tag`] through [`Body::tag`], and the wire carries
/// both: a body whose discriminant disagrees with the header tag is malformed.
/// Variants whose header slot carries no protocol meaning are encoded with the
/// sentinel `Slot(0)` by their senders; the rule is
/// [`crate::invariant::header_slot_role`]'s, stated there and not repeated here.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Body {
    /// Client operation submitted to the primary (§6). The payload is opaque:
    /// the core stores and carries it and never inspects it (§11).
    Request {
        /// The client identity, for the duplicate-suppression table.
        client: ClientId,
        /// The client's monotonic request number.
        request: RequestNumber,
        /// Opaque application bytes.
        payload: Box<[u8]>,
    },
    /// The primary's proposal of `entry` at its slot (§6), with the sender's
    /// commit frontier piggybacked so a backup learns of commits without a
    /// separate round (§13.3).
    Prepare {
        /// The proposed entry; its slot is the header slot.
        entry: LogEntry,
        /// The primary's committed frontier at send time.
        committed: Slot,
    },
    /// A backup's acceptance of a `Prepare` (§6). The accepted slot is the
    /// header slot; the body is empty because acceptance claims nothing else.
    PrepareOk {},
    /// A commit-frontier advance carrying no new operation (§6, §13.3).
    Commit {
        /// The committed frontier the message advances the recipient to.
        committed: Slot,
    },
    /// The ordinary view-change fence (§9.1): a recipient stops accepting
    /// `Prepare` in the old view. The target view is the header view. Distinct
    /// from [`Body::PlannedViewChange`], which must not fence.
    StartViewChange {},
    /// A replica's state evidence for the designated new primary (§9.1): the
    /// retained history's provenance, its frontiers, the suffix the new primary
    /// lacks, the sender's volatile client table (§9.2 — the table is protocol
    /// evidence: without it a retry of a committed request whose result died
    /// with the old primary would be accepted as new and appended twice), and
    /// the proof that the sender may speak for its era (§8.7.8).
    DoViewChange {
        /// The view at which the reported history was selected (§1.3, §9.1's
        /// ranking rule).
        retained: ViewId,
        /// The reported history's accepted frontier; also the header slot.
        accepted: Slot,
        /// The reported history's committed frontier.
        committed: Slot,
        /// The uncommitted tail the new primary may need, bounded under §13.1.
        suffix: Vec<LogEntry>,
        /// The sender's client table, sorted by client with no duplicate
        /// clients. Counts against the §13.1 budget together with the suffix;
        /// the table is charged first and is never truncated (it is safety
        /// evidence; the suffix is reconstructible via the §13.1 step-5
        /// fetch).
        client_table: Vec<ClientRow>,
        /// Whether this evidence is ordinary or planned (§8.7.7); planned
        /// evidence is not a fence.
        evidence: EvidenceKind,
        /// The sender's entitlement to speak for its era.
        era_proof: EraProof,
    },
    /// The new primary installing the selected history (§9.1, §13.1).
    StartView {
        /// The selected history's tail, from the recipient's frontier onward.
        suffix: Vec<LogEntry>,
        /// The installed history's accepted frontier; also the header slot.
        accepted: Slot,
        /// The installed history's committed frontier.
        committed: Slot,
        /// The merged client table of the evidence quorum (§9.2): per client
        /// the row with the greatest `last_request`, ties broken toward the
        /// row WITH a cached result. Recipients install it in place of their
        /// own — the merged table dominates because it saw a view-change
        /// quorum's rows.
        client_table: Vec<ClientRow>,
        /// The sender's entitlement to speak for its era (§8.7.8).
        era_proof: EraProof,
    },
    /// Solicitation of planned view-change evidence during overlap mode
    /// (§8.7.7 step 4). NOT a fence: the recipient keeps accepting `Prepare`
    /// in the current view, which is exactly why this is a distinct tag rather
    /// than a flag on `StartViewChange` — see [`Tag::PlannedViewChange`].
    PlannedViewChange {},
    /// A restarting replica soliciting state (§10). `nonce` IS the host tick
    /// of the recovery input (S4): one value cannot disagree with itself, and
    /// the §6.1 freshness obligation is the host's declared clock strategy.
    Recovery {
        /// The attempt's nonce: the `TimedInput.at` of the recovery event.
        nonce: Tick,
    },
    /// A reply to [`Body::Recovery`], echoing the nonce so a delayed response
    /// from an earlier attempt is never counted in the current one (§6.1).
    RecoveryResponse {
        /// The nonce of the attempt being answered.
        nonce: Tick,
        /// The responder's accepted frontier; also the header slot.
        accepted: Slot,
        /// The responder's committed frontier.
        committed: Slot,
        /// The history suffix — present only from the primary of the view,
        /// whose log is the authoritative one for the attempt (§10).
        suffix: Option<Vec<LogEntry>>,
    },
    /// A request for the history range the requester lacks (§4, §13.1).
    GetState {
        /// The first slot the requester needs: one past its accepted frontier.
        from: Slot,
    },
    /// A history range in reply to [`Body::GetState`] (§4). Sizing is the
    /// host's packetization decision (W5).
    NewState {
        /// The transferred entries, contiguous and slot-ordered.
        entries: Vec<LogEntry>,
        /// The last slot the range covers; also the header slot.
        through: Slot,
        /// The sender's committed frontier at send time.
        committed: Slot,
    },
    /// The primary's reply to a client (§6, §11.2): emitted only after the
    /// corresponding application effect completed.
    Reply {
        /// The client being answered.
        client: ClientId,
        /// The request being answered.
        request: RequestNumber,
        /// The application's result bytes, opaque to the core (§11).
        result: Box<[u8]>,
    },
}

/// Whether view-change evidence is ordinary or planned (§8.7.7).
///
/// The distinction decides which quorum the new primary is completing, but it
/// does not fence the sender or the recipient — that is why it is a body field
/// and not a tag, in contrast to [`Tag::PlannedViewChange`], whose entire
/// content is the fencing difference.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EvidenceKind {
    /// Evidence produced by an ordinary §9.1 view change.
    Ordinary,
    /// Evidence solicited by [`Tag::PlannedViewChange`] during overlap mode
    /// (§8.7.7): it completes the planned quorum and fences nothing.
    Planned,
}

/// Proof the sender is entitled to speak for an era (§8.7.8): the operation
/// that established the era and the slot at which that operation committed.
///
/// Era and establishing operation are in one-to-one correspondence (§8.7.1's
/// fold), so carrying the operation and its committed slot is the complete
/// claim; the recipient checks it against its own configuration history rather
/// than trusting the sender's arithmetic.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EraProof {
    /// The reconfiguration operation that established the era.
    pub op: SystemOperation,
    /// The slot at which `op` committed.
    pub committed_at: Slot,
}

impl Body {
    /// The tag this body is the payload of.
    ///
    /// Total and bijective over the alphabet: every body names exactly one tag,
    /// and the decoder refuses a body whose tag disagrees with the header's.
    #[must_use]
    pub fn tag(&self) -> Tag {
        match self {
            Body::Request { .. } => Tag::Request,
            Body::Prepare { .. } => Tag::Prepare,
            Body::PrepareOk {} => Tag::PrepareOk,
            Body::Commit { .. } => Tag::Commit,
            Body::StartViewChange {} => Tag::StartViewChange,
            Body::DoViewChange { .. } => Tag::DoViewChange,
            Body::StartView { .. } => Tag::StartView,
            Body::PlannedViewChange {} => Tag::PlannedViewChange,
            Body::Recovery { .. } => Tag::Recovery,
            Body::RecoveryResponse { .. } => Tag::RecoveryResponse,
            Body::GetState { .. } => Tag::GetState,
            Body::NewState { .. } => Tag::NewState,
            Body::Reply { .. } => Tag::Reply,
        }
    }

    /// The wire discriminant: the tag's numbering narrowed to one byte.
    ///
    /// The `expect` is unreachable by construction: [`Tag::as_u32`] yields
    /// 1..=13, and the conversion is a `try_from` rather than a cast because
    /// the crate forbids `as` between integer widths — a tag added past 255
    /// fails loudly here instead of truncating onto the wire.
    fn discriminant(&self) -> u8 {
        u8::try_from(self.tag().as_u32()).expect("tag discriminants are 1..=13")
    }
}

/// Encoded length of a `u32`-counted entry sequence.
fn entries_packed_len(entries: &[LogEntry]) -> usize {
    // A `Vec` cannot hold enough variable-width entries for the sum to
    // overflow `usize`: each entry is at least ten bytes of the same memory.
    4 + entries.iter().map(Pack::packed_len).sum::<usize>()
}

/// Writes a `u32` count and then each entry.
fn pack_entries(entries: &[LogEntry], w: &mut PackWriter<'_>) {
    let count =
        u32::try_from(entries.len()).expect("an entry sequence beyond u32::MAX cannot be framed");
    w.u32(count);
    for entry in entries {
        entry.pack(w);
    }
}

/// Reads a `u32`-counted entry sequence.
///
/// No `with_capacity(count)`: the count is untrusted input, and reserving
/// against it would let a four-byte prefix demand an unbounded allocation.
/// Growth is bounded by the bytes actually present, because every entry decode
/// consumes input or fails.
fn unpack_entries(c: &mut UnpackCursor<'_>) -> Result<Vec<LogEntry>, UnpackError> {
    let count = c.u32()?;
    let mut entries = Vec::new();
    for _ in 0..count {
        entries.push(LogEntry::unpack(c)?);
    }
    Ok(entries)
}

impl Pack for EraProof {
    fn packed_len(&self) -> usize {
        self.op.packed_len() + self.committed_at.packed_len()
    }

    fn pack(&self, w: &mut PackWriter<'_>) {
        self.op.pack(w);
        self.committed_at.pack(w);
    }
}

impl Unpack for EraProof {
    fn unpack(c: &mut UnpackCursor<'_>) -> Result<Self, UnpackError> {
        let op = SystemOperation::unpack(c)?;
        let committed_at = Slot::unpack(c)?;
        Ok(EraProof { op, committed_at })
    }
}

impl Pack for EvidenceKind {
    fn packed_len(&self) -> usize {
        1
    }

    fn pack(&self, w: &mut PackWriter<'_>) {
        // `0` reserved, on the established rule: an all-zero buffer is
        // malformed, never a valid `Ordinary`.
        w.u8(match self {
            EvidenceKind::Ordinary => 1,
            EvidenceKind::Planned => 2,
        });
    }
}

impl Unpack for EvidenceKind {
    fn unpack(c: &mut UnpackCursor<'_>) -> Result<Self, UnpackError> {
        match c.u8()? {
            1 => Ok(EvidenceKind::Ordinary),
            2 => Ok(EvidenceKind::Planned),
            _ => Err(UnpackError::Malformed(Malformed::OutOfDomain)),
        }
    }
}

// ---------------------------------------------------------------------------
// Client-table rows
// ---------------------------------------------------------------------------

/// One client-table row as the view-change evidence carries it (§9.2): the
/// greatest request accepted from this client, and the cached reply if the
/// volatile result is still at hand.
///
/// The table is protocol evidence (§9.2), riding `DoViewChange` and
/// `StartView`: without it the new primary's table would be empty, and a
/// retry of a committed request whose result died with the old primary
/// would be accepted as new — a second log entry for one logical request.
/// The result is best-effort: a row without one is the unknown-result case,
/// which the new primary answers by re-driving `Effect::Apply` for the
/// committed slot (§11 puts idempotence on the host).
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ClientRow {
    /// The client this row belongs to.
    pub client: ClientId,
    /// The greatest request number accepted from this client.
    pub last_request: RequestNumber,
    /// The cached reply, if the volatile result is still at hand.
    pub result: Option<Box<[u8]>>,
}

impl Pack for ClientRow {
    fn packed_len(&self) -> usize {
        let result = match &self.result {
            Some(bytes) => opaque_packed_len(bytes),
            None => 0,
        };
        self.client.packed_len() + self.last_request.packed_len() + 1 + result
    }

    fn pack(&self, w: &mut PackWriter<'_>) {
        self.client.pack(w);
        self.last_request.pack(w);
        match &self.result {
            Some(bytes) => {
                w.u8(1);
                w.opaque(bytes);
            }
            None => w.u8(0),
        }
    }
}

impl Unpack for ClientRow {
    fn unpack(c: &mut UnpackCursor<'_>) -> Result<Self, UnpackError> {
        let client = ClientId::unpack(c)?;
        let last_request = RequestNumber::unpack(c)?;
        let result = match c.u8()? {
            0 => None,
            1 => Some(c.opaque()?.to_vec().into_boxed_slice()),
            _ => return Err(UnpackError::Malformed(Malformed::OutOfDomain)),
        };
        Ok(ClientRow {
            client,
            last_request,
            result,
        })
    }
}

/// Packs a client table as `u32` count then each row in order. The producer
/// emits rows sorted by client with no duplicates; decoding validates that
/// (strictly ascending ids), so a malformed or out-of-order table is
/// rejected at the boundary rather than trusted into a merge.
fn pack_client_table(table: &[ClientRow], w: &mut PackWriter<'_>) {
    w.u32(u32::try_from(table.len()).expect("a client table fits in u32"));
    for row in table {
        row.pack(w);
    }
}

/// The packed length of the codec form of [`pack_client_table`].
fn client_table_packed_len(table: &[ClientRow]) -> usize {
    4 + table.iter().map(Pack::packed_len).sum::<usize>()
}

/// Decodes a client table, requiring strictly ascending client ids (hence
/// no duplicates) — the producer's canonical form, checked at the boundary.
fn unpack_client_table(c: &mut UnpackCursor<'_>) -> Result<Vec<ClientRow>, UnpackError> {
    let count = c.u32()?;
    let mut table = Vec::with_capacity(usize::try_from(count).expect("u32 fits in usize"));
    let mut previous: Option<ClientId> = None;
    for _ in 0..count {
        let row = ClientRow::unpack(c)?;
        if previous.is_some_and(|id| row.client <= id) {
            return Err(UnpackError::Malformed(Malformed::OutOfDomain));
        }
        previous = Some(row.client);
        table.push(row);
    }
    Ok(table)
}

impl Pack for Body {
    fn packed_len(&self) -> usize {
        // One byte of discriminant, then fixed-width fields (W4). Every length
        // is an exact sum, per the normative W3 contract the §13.1 suffix
        // budget relies on.
        let fields = match self {
            Body::Request {
                client,
                request,
                payload,
            } => client.packed_len() + request.packed_len() + opaque_packed_len(payload),
            Body::Prepare { entry, committed } => entry.packed_len() + committed.packed_len(),
            Body::PrepareOk {} | Body::StartViewChange {} | Body::PlannedViewChange {} => 0,
            Body::Commit { committed } => committed.packed_len(),
            Body::DoViewChange {
                retained,
                accepted,
                committed,
                suffix,
                client_table,
                evidence,
                era_proof,
            } => {
                retained.packed_len()
                    + accepted.packed_len()
                    + committed.packed_len()
                    + entries_packed_len(suffix)
                    + client_table_packed_len(client_table)
                    + evidence.packed_len()
                    + era_proof.packed_len()
            }
            Body::StartView {
                suffix,
                accepted,
                committed,
                client_table,
                era_proof,
            } => {
                entries_packed_len(suffix)
                    + accepted.packed_len()
                    + committed.packed_len()
                    + client_table_packed_len(client_table)
                    + era_proof.packed_len()
            }
            Body::Recovery { nonce } => nonce.packed_len(),
            Body::RecoveryResponse {
                nonce,
                accepted,
                committed,
                suffix,
            } => {
                nonce.packed_len()
                    + accepted.packed_len()
                    + committed.packed_len()
                    + 1
                    + match suffix {
                        Some(entries) => entries_packed_len(entries),
                        None => 0,
                    }
            }
            Body::GetState { from } => from.packed_len(),
            Body::NewState {
                entries,
                through,
                committed,
            } => entries_packed_len(entries) + through.packed_len() + committed.packed_len(),
            Body::Reply {
                client,
                request,
                result,
            } => client.packed_len() + request.packed_len() + opaque_packed_len(result),
        };
        1 + fields
    }

    fn pack(&self, w: &mut PackWriter<'_>) {
        w.u8(self.discriminant());
        match self {
            Body::Request {
                client,
                request,
                payload,
            } => {
                client.pack(w);
                request.pack(w);
                w.opaque(payload);
            }
            Body::Prepare { entry, committed } => {
                entry.pack(w);
                committed.pack(w);
            }
            Body::PrepareOk {} | Body::StartViewChange {} | Body::PlannedViewChange {} => {}
            Body::Commit { committed } => committed.pack(w),
            Body::DoViewChange {
                retained,
                accepted,
                committed,
                suffix,
                client_table,
                evidence,
                era_proof,
            } => {
                retained.pack(w);
                accepted.pack(w);
                committed.pack(w);
                pack_entries(suffix, w);
                pack_client_table(client_table, w);
                evidence.pack(w);
                era_proof.pack(w);
            }
            Body::StartView {
                suffix,
                accepted,
                committed,
                client_table,
                era_proof,
            } => {
                pack_entries(suffix, w);
                accepted.pack(w);
                committed.pack(w);
                pack_client_table(client_table, w);
                era_proof.pack(w);
            }
            Body::Recovery { nonce } => nonce.pack(w),
            Body::RecoveryResponse {
                nonce,
                accepted,
                committed,
                suffix,
            } => {
                nonce.pack(w);
                accepted.pack(w);
                committed.pack(w);
                match suffix {
                    Some(entries) => {
                        w.bool(true);
                        pack_entries(entries, w);
                    }
                    None => w.bool(false),
                }
            }
            Body::GetState { from } => from.pack(w),
            Body::NewState {
                entries,
                through,
                committed,
            } => {
                pack_entries(entries, w);
                through.pack(w);
                committed.pack(w);
            }
            Body::Reply {
                client,
                request,
                result,
            } => {
                client.pack(w);
                request.pack(w);
                w.opaque(result);
            }
        }
    }
}

impl Unpack for Body {
    fn unpack(c: &mut UnpackCursor<'_>) -> Result<Self, UnpackError> {
        // Discriminant first, so an all-zero buffer fails before any field is
        // read. `0` is reserved and everything above the table is unknown;
        // both are `OutOfDomain`, not `UnknownTag`, because that variant
        // carries the header's `u32` tag and this discriminant is a `u8`.
        let tag = match c.u8()? {
            1 => Tag::Request,
            2 => Tag::Prepare,
            3 => Tag::PrepareOk,
            4 => Tag::Commit,
            5 => Tag::StartViewChange,
            6 => Tag::DoViewChange,
            7 => Tag::StartView,
            8 => Tag::PlannedViewChange,
            9 => Tag::Recovery,
            10 => Tag::RecoveryResponse,
            11 => Tag::GetState,
            12 => Tag::NewState,
            13 => Tag::Reply,
            _ => return Err(UnpackError::Malformed(Malformed::OutOfDomain)),
        };
        let body = match tag {
            Tag::Request => Body::Request {
                client: ClientId::unpack(c)?,
                request: RequestNumber::unpack(c)?,
                // Borrowed from the cursor and copied only here, at the
                // boundary where the message takes ownership (§11).
                payload: c.opaque()?.to_vec().into_boxed_slice(),
            },
            Tag::Prepare => Body::Prepare {
                entry: LogEntry::unpack(c)?,
                committed: Slot::unpack(c)?,
            },
            Tag::PrepareOk => Body::PrepareOk {},
            Tag::Commit => Body::Commit {
                committed: Slot::unpack(c)?,
            },
            Tag::StartViewChange => Body::StartViewChange {},
            Tag::DoViewChange => Body::DoViewChange {
                retained: ViewId::unpack(c)?,
                accepted: Slot::unpack(c)?,
                committed: Slot::unpack(c)?,
                suffix: unpack_entries(c)?,
                client_table: unpack_client_table(c)?,
                evidence: EvidenceKind::unpack(c)?,
                era_proof: EraProof::unpack(c)?,
            },
            Tag::StartView => Body::StartView {
                suffix: unpack_entries(c)?,
                accepted: Slot::unpack(c)?,
                committed: Slot::unpack(c)?,
                client_table: unpack_client_table(c)?,
                era_proof: EraProof::unpack(c)?,
            },
            Tag::PlannedViewChange => Body::PlannedViewChange {},
            Tag::Recovery => Body::Recovery {
                nonce: Tick::unpack(c)?,
            },
            Tag::RecoveryResponse => Body::RecoveryResponse {
                nonce: Tick::unpack(c)?,
                accepted: Slot::unpack(c)?,
                committed: Slot::unpack(c)?,
                suffix: match c.bool()? {
                    true => Some(unpack_entries(c)?),
                    false => None,
                },
            },
            Tag::GetState => Body::GetState {
                from: Slot::unpack(c)?,
            },
            Tag::NewState => Body::NewState {
                entries: unpack_entries(c)?,
                through: Slot::unpack(c)?,
                committed: Slot::unpack(c)?,
            },
            Tag::Reply => Body::Reply {
                client: ClientId::unpack(c)?,
                request: RequestNumber::unpack(c)?,
                result: c.opaque()?.to_vec().into_boxed_slice(),
            },
        };
        Ok(body)
    }
}

impl Pack for Message {
    fn packed_len(&self) -> usize {
        self.header.packed_len() + self.body.packed_len()
    }

    fn pack(&self, w: &mut PackWriter<'_>) {
        self.header.pack(w);
        self.body.pack(w);
    }
}

impl Unpack for Message {
    fn unpack(c: &mut UnpackCursor<'_>) -> Result<Self, UnpackError> {
        let header = Header::unpack(c)?;
        let body = Body::unpack(c)?;
        // The kind is carried twice; a disagreement is malformed rather than
        // guessed at, because the wire cannot say which field lied.
        if body.tag() != header.tag {
            return Err(UnpackError::Malformed(Malformed::OutOfDomain));
        }
        Ok(Message { header, body })
    }
}
