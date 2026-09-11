//! Identity: node identifiers, view identifiers, slots, operation identities.
//!
//! Spec §1.2 (view and configuration generation), §1.3 (slots and frontiers), and
//! Amendment A1 / decision W1, which supersede the packed `view = (era << k) | index`
//! encoding of §8.7.3.
//!
//! A view is an explicit `ViewId { era: u32, view: u32 }`. Two independent fields
//! cannot alias, so §8.7.3's checked-encoding requirement and its prohibition on
//! wraparound are discharged by construction rather than by validation, and a host can
//! route on era without decoding a protocol number.
//!
//! Identifiers are supplied by the host (decision W2): `OperationId` is 128
//! host-chosen bits the core carries opaque. The core never mints one, so it needs
//! no randomness source and no `uuid` dependency; correlation is whatever the
//! host's naming discipline says it is.
//!
//! Ordering on identifiers is total and lexicographic in `(era, view)`, because the
//! higher-view rule of §10 must be decidable from the header alone, without consulting
//! configuration state that a fenced or recovering replica may not yet hold.
//!
//! # Why newtypes rather than aliases
//!
//! `type View = u32` is naming, not typing: it lets a slot be passed where a view is
//! expected, and the resulting bug is a silent safety violation rather than a type
//! error. Each identifier below is a distinct `#[repr(transparent)]` newtype, so the
//! confusion is rejected at compile time while the C ABI still passes the
//! value as the bare primitive with no marshalling layer to disagree with itself. The
//! `const` layout assertions exist so a later added field or changed repr is a
//! compile error here rather than an ABI break discovered by a host.
//!
//! # Why every successor returns `Option`
//!
//! §8.7.3 forbids wraparound outright. A wrapped view number reuses a primary term and
//! a wrapped slot reuses a log position; both are unrecoverable divergence, not a
//! recoverable error. There is therefore no `+ 1`, no `wrapping_*`, and no `as` cast
//! between integer widths anywhere in this module: the exhaustion of a 32-bit view
//! space is a state the caller must handle explicitly.
//!
//! # Why no `Default`
//!
//! `Era`, `View`, `Slot`, `NodeId` and `Tick` deliberately do not implement `Default`.
//! A defaulted identifier is a value nobody chose, and any of these can reach a journal
//! record or a published `Progress` (§5), where "zero because nobody set it" and "zero
//! because that is the genesis value" are indistinguishable after the fact. Where a base
//! value is genuinely meaningful it is a named constant with documented meaning:
//! [`Era::INITIAL`], [`View::INITIAL`], [`Slot::NONE`]. `NodeId` and `Tick` get none,
//! because no node identifier is privileged (§8.7.1: `order` is host-assigned) and no
//! tick is (S4: every tick comes from the host).
//!
//! # Why the serde derives are `cfg_attr` and not `derive`
//!
//! Each type below carries `#[cfg_attr(feature = "serde", derive(...))]` so the default
//! build contains no trace of serde and a consumer's non-optional dependency set stays
//! empty (W3, enforced by `tests/manifest_contract.rs`). Serde exists for a host that
//! wants its own encoding and for the `maelstrom` debug bridge. It is never the wire
//! format: the normative encoding is the binary codec in [`crate::wire`], because only a
//! fixed-width encoding makes the §13.1 suffix budget an exact sum (W4).

use core::mem::{align_of, size_of};

/// Replica identity within a configuration.
///
/// Assigned by the system administrator and opaque to the core: §8.7.1 makes `order` a
/// host-supplied list of distinct identifiers, and node naming is an open extension
/// point, so the core never derives, orders by, or assigns meaning to the integer
/// beyond equality and a total order for deterministic iteration.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NodeId(pub u32);

const _: () = assert!(size_of::<NodeId>() == size_of::<u32>());
const _: () = assert!(align_of::<NodeId>() == align_of::<u32>());

/// Configuration generation.
///
/// Spec §8.7.1: era `e` indexes the configuration sequence, and `config(e)` is the fold
/// of committed reconfiguration operations through the operation establishing `e`. It is
/// a separate field from [`View`] and is never packed into one (W1), so a host can
/// dispatch overlap-mode traffic on era without decoding a protocol number.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Era(pub u32);

const _: () = assert!(size_of::<Era>() == size_of::<u32>());
const _: () = assert!(align_of::<Era>() == align_of::<u32>());

/// Primary-succession number, and nothing else.
///
/// Spec §1.2 and §15 item 2: `view` selects the primary via `primary(v) = order[v mod
/// N]`. Membership and quorum-policy changes advance [`Era`], never this. View-number
/// gaps are legal (§8.7.3), so a successor is not required to be the immediate integer
/// successor of its predecessor.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct View(pub u32);

const _: () = assert!(size_of::<View>() == size_of::<u32>());
const _: () = assert!(align_of::<View>() == align_of::<u32>());

/// Log position.
///
/// Spec §1.3: the `applied <= committed <= accepted` frontiers are all slots. A slot is
/// assigned once in a legitimate history, which is why [`Slot::next`] is checked rather
/// than wrapping.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Slot(pub u64);

const _: () = assert!(size_of::<Slot>() == size_of::<u64>());
const _: () = assert!(align_of::<Slot>() == align_of::<u64>());

/// Host-supplied event value carried by every input.
///
/// Spec §6.1 and decision S4: the core reads no clock, and for a restart input this
/// value **is** the restart nonce. One value cannot disagree with itself, which is why
/// there is no separate nonce type. The core treats it as opaque and converts no units,
/// so nanoseconds, milliseconds and a harness counter are all admissible; the §6.1
/// obligation that no restart attempt reuse a nonce while an earlier attempt's message
/// can still be delivered is a property of the host's declared clock strategy and is not
/// checkable here.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Tick(pub u64);

const _: () = assert!(size_of::<Tick>() == size_of::<u64>());
const _: () = assert!(align_of::<Tick>() == align_of::<u64>());

/// An operation's identity at the application boundary (§11.1, B2): 128
/// bits the proposing host assigns, and the core carries opaque — proposed
/// with the operation, replicated inside its log entry, and handed back on
/// `Effect::Apply`. The core never generates one, never inspects one, and
/// never deduplicates on one: the same identity proposed twice is two
/// operations. Deduplication is the host's policy above the boundary.
///
/// Two `u64` words rather than a `u128` or a byte array: the words are the
/// host's name for the operation (a split such as `{client, sequence}` is
/// the host's scheme, invisible here), the pair is `Ord`, `Hash` and `Copy`
/// with machine-level comparisons, and the W3 wire codec fixes the byte
/// order — most significant word first, both words big-endian — at the one
/// place byte order is a decision rather than an accident.
///
/// `#[repr(C)]` so the layout is as fixed as the newtypes': `msb` at offset
/// 0, `lsb` at offset 8, no padding — a host handing the identity across an
/// FFI boundary reads the words where it wrote them.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OperationId {
    /// The most significant 64 bits of the identity.
    pub msb: u64,
    /// The least significant 64 bits of the identity.
    pub lsb: u64,
}

const _: () = assert!(size_of::<OperationId>() == 2 * size_of::<u64>());
const _: () = assert!(align_of::<OperationId>() == align_of::<u64>());

/// One host operation at the application boundary (§11.1, B2): the identity
/// the proposing host assigned and the opaque bytes to be ordered. This is
/// the unit `Input::Propose` carries; commitment hands its parts back on
/// `Effect::Apply` — same identity, same bytes, at every replica.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Operation {
    /// The operation's identity, host-assigned (§11.1). Opaque to the core.
    pub id: OperationId,
    /// Opaque application bytes. The core stores and carries them, and
    /// never inspects them (§11.1).
    pub payload: Box<[u8]>,
}

impl Era {
    /// The era established at provisioning, before any reconfiguration operation has
    /// committed. Named rather than defaulted: this is the era of the genesis
    /// configuration (§8.7.1), not a placeholder for an unknown era.
    pub const INITIAL: Era = Era(0);

    /// The next configuration generation, or `None` if the era space is exhausted.
    ///
    /// §8.7.3 forbids wraparound: a reused era would make `config(e)` ambiguous and
    /// break the `R1`/`R2` intersection argument (§8.7.4), which is stated over
    /// consecutive eras.
    #[must_use]
    pub fn next(self) -> Option<Era> {
        self.0.checked_add(1).map(Era)
    }
}

impl View {
    /// The view a freshly provisioned replica starts in. Named rather than defaulted:
    /// `primary(0) = order[0]` is a real protocol fact (§1.2), not an absence of
    /// information.
    pub const INITIAL: View = View(0);

    /// The immediately following view, or `None` if the view space is exhausted.
    ///
    /// Only the *immediate* successor. View gaps are legal (§8.7.3) and a planned view
    /// change chooses its target with [`next_view_selecting`] instead, so this is the
    /// ordinary timeout-driven step and not the general succession rule.
    #[must_use]
    pub fn next(self) -> Option<View> {
        self.0.checked_add(1).map(View)
    }
}

impl Slot {
    /// The crate-wide no-slot sentinel: strictly below the first history position
    /// (`crate::configuration::VOID_SLOT`, slot 1), at a position that can never hold
    /// an entry. Named rather than defaulted, and named `NONE` rather than `ZERO`
    /// because §1.3's frontiers are positions in a history, so the absence of a
    /// position is a protocol fact while the integer that represents it is not.
    pub const NONE: Slot = Slot(0);

    /// The next log position, or `None` if the slot space is exhausted.
    ///
    /// A wrapped slot would reassign a position that already holds an accepted
    /// operation, so exhaustion is reported rather than absorbed.
    #[must_use]
    pub fn next(self) -> Option<Slot> {
        self.0.checked_add(1).map(Slot)
    }

    /// The preceding log position, or `None` at [`Slot::NONE`].
    ///
    /// `None` rather than a saturating sentinel because "the slot before the
    /// sentinel" is not a position, and a caller walking a history backwards must
    /// terminate on the absence rather than loop on a fixed point.
    #[must_use]
    pub fn prev(self) -> Option<Slot> {
        self.0.checked_sub(1).map(Slot)
    }

    /// The number of positions from `self` up to `other`, or `None` if `other` is
    /// behind `self`.
    ///
    /// Suffix-length arithmetic for §13.1's bounded view-change suffix and for state
    /// transfer ranges. It saturates nowhere: a negative length coerced to zero would
    /// make a truncated suffix indistinguishable from a complete one, and the caller
    /// that asked for a backwards range has a bug the core must not hide. Reflexive
    /// distance is `0`, so the result counts steps, not elements.
    #[must_use]
    pub fn distance_to(self, other: Slot) -> Option<u64> {
        other.0.checked_sub(self.0)
    }
}

/// A view together with the configuration generation that authorises it.
///
/// Spec §1.2, §8.7.3, Amendment A1, decision W1. Two independent `u32` fields replace
/// the packed `view = (era << k) | index` encoding, so no genesis-time choice of `k`
/// can bound era and index simultaneously, and no host has to decode a protocol number
/// to make a transport decision.
///
/// # Field order, and the ordering claim it rests on
///
/// The order `{ era, view }` is deliberate: the derived lexicographic [`Ord`] is
/// therefore `(era, view)`.
///
/// On any legal pair the orderings `(era, view)` and `(view, era)` coincide, because era
/// advances only as views advance — a higher era implies a higher view within any single
/// legitimate history. Pairs on which the two orderings disagree are illegal and are
/// rejected by `invariant`, not silently ordered. `Ord` is therefore a total order that
/// is only *meaningful* on validated pairs, and callers must validate before they
/// compare across nodes.
///
/// That claim is discharged by `tests/ids_contract.rs`, not by this comment.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ViewId {
    /// The configuration generation authorising this view (§8.7.1).
    pub era: Era,
    /// The primary-succession number within that generation (§1.2).
    pub view: View,
}

impl ViewId {
    /// The **genesis view**: [`Era::INITIAL`] and [`View::INITIAL`] — the view a
    /// freshly provisioned node advertises, pinned by `Progress::genesis` (the
    /// genesis ruling, §1.3). Era 0 is the void configuration, quorum-impossible by arithmetic,
    /// and view 0 is the first primary term once `Init` commits (§1.2:
    /// `primary(0) = order[0]`). It is not "no view": a freshly provisioned node
    /// has a real genesis view, so no `Option<ViewId>` appears anywhere. Named
    /// rather than a `Default` impl, because a `ViewId` that nobody chose can
    /// reach a journal record and a `StartView` message, where it is
    /// indistinguishable from the genesis view.
    pub const INITIAL: ViewId = ViewId {
        era: Era::INITIAL,
        view: View::INITIAL,
    };

    /// Next view in the same era, or `None` on [`View`] overflow.
    ///
    /// The ordinary view change: the configuration is unchanged, so only the primary
    /// term advances (§9.1).
    #[must_use]
    pub fn next_in_era(self) -> Option<ViewId> {
        self.view.next().map(|view| ViewId {
            era: self.era,
            view,
        })
    }

    /// Next view in the next era, or `None` on either overflow.
    ///
    /// The pivot of §8.7.7 step 3: the primary picks `v' > v` with `era(v') = e+1` once
    /// the reconfiguration establishing `e+1` has committed. Both fields are checked
    /// independently because they cannot alias (W1), so exhausting one must not silently
    /// alter the other.
    #[must_use]
    pub fn next_in_next_era(self) -> Option<ViewId> {
        match (self.era.next(), self.view.next()) {
            (Some(era), Some(view)) => Some(ViewId { era, view }),
            (None, _) | (_, None) => None,
        }
    }

    /// Is `next` a legal successor of `self`?
    ///
    /// True exactly when `next.view > self.view` and `next.era` is either `self.era` or
    /// `self.era + 1`. This is §8.7.3's surviving relation `era(view) <= era(slot) <=
    /// era(view) + 1` read across a view change: a view change strictly increases the
    /// view, while the era either stands still or advances by exactly one, because era
    /// `e+1` is established by a single committed reconfiguration operation. View gaps
    /// are legal, so the view increase need not be by one.
    ///
    /// This is the single point of truth for the rule. Callers use it; they do not
    /// re-derive it, because two copies of an inequality are two chances to get it
    /// wrong. Note the asymmetry with the era: an era jump of two is refused even though
    /// a view jump of two is accepted, since skipping an era means skipping a
    /// configuration whose intersection obligations were never checked (§8.7.4, Q1).
    #[must_use]
    pub fn is_legal_successor(self, next: ViewId) -> bool {
        let view_advances = next.view > self.view;
        let era_legal = match self.era.next() {
            Some(era_plus_one) => next.era == self.era || next.era == era_plus_one,
            // The era space is exhausted, so only "era unchanged" remains legal.
            None => next.era == self.era,
        };
        view_advances && era_legal
    }
}

/// Least `v' > current` with `v' % members == index`, or `None` on [`View`] overflow.
///
/// This is the arithmetic a primary uses to choose its own next view during a planned
/// view change (spec §8.7.7 step 4). It knows nothing about membership; the caller
/// supplies `index` and `members` from the configuration it has already validated, which
/// is why this lives in `ids` and not in `configuration` — it is modular arithmetic over
/// §1.2's `primary(v) = order[v mod N]`, with no notion of who occupies the slot.
///
/// The result normally skips views. That is correct: §8.7.3 states that view-number gaps
/// are legal, and the skipped views are precisely those that would select some other
/// member.
///
/// Returns `None` rather than panicking when `members == 0` or `index >= members`. Both
/// are unsatisfiable rather than erroneous — no view selects a member of an empty
/// configuration — and the core does not abort a host process over an argument it can
/// refuse. `None` is also returned when the least satisfying view is not representable;
/// there is no wraparound (§8.7.3), so a cluster that exhausts the view space stops
/// rather than reuses a primary term.
#[must_use]
pub fn next_view_selecting(current: View, index: u32, members: u32) -> Option<View> {
    if members == 0 || index >= members {
        return None;
    }

    // Advance past `current` first, so the answer is strictly greater even when
    // `current` already selects `index`.
    let floor = current.0.checked_add(1)?;

    // Distance from `floor` up to the next view congruent to `index` modulo `members`,
    // i.e. `(index - residue) mod members`, split into the two cases so the subtraction
    // never goes negative. Every intermediate stays inside `0..members`, so neither
    // branch can overflow whatever `members` the host supplied, and the only
    // representability question left is the final addition.
    let residue = floor % members;
    let gap = if residue <= index {
        index - residue
    } else {
        members - (residue - index)
    };

    let selecting = floor.checked_add(gap)?;
    Some(View(selecting))
}

/// Why a node has declared itself unfit to participate.
///
/// A fault is **sticky**: once set it is never cleared, and a faulted node refuses all
/// new input, forcing a host restart and an explicit restart that establishes a coherent
/// state (§5 invariant 5, §12's `Indeterminate persistence result -> Faulted`). The
/// variants below are the complete set of sources, enumerated rather than collapsed into
/// a string, so that a `match` over them is a compile-time obligation for every consumer
/// and a new source cannot be introduced without classifying it.
///
/// The distinction that matters most here is the one S3 draws: a *determinate*
/// persistence failure is a normal, survivable event and is **not** a fault. Only an
/// outcome the host could not determine is, because that is the only case in which the
/// node cannot say what its own durable state is.
///
/// This enum lives in `ids`, not in `invariant`, by architectural ruling: `Fault` names
/// fault *kinds* and is identity-level state carried inside `Progress`. Placing it in
/// `invariant` would close a module cycle — `invariant::legal` consumes `Progress`
/// while `progress` would have to import `Fault` back from `invariant` — and the
/// architecture prohibits cycles. `invariant` re-exports it so existing citations keep
/// compiling.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Fault {
    /// A planned transition would have produced a state unreachable from the
    /// current one. Spec §5, §15.
    IllegalTransition,
    /// A persistence attempt returned an outcome the host could not determine
    /// (S3, spec §5 invariant 5). A *determinate* failure is NOT a fault.
    IndeterminatePersistence,
    /// The declared quorum strategy failed a closed intersection obligation
    /// (Q1: R1, R2, §8.3).
    QuorumObligation,
    /// Progress and journal disagreed about the accepted frontier.
    ProgressJournalDivergence,
    /// The host declared this node unfit (spec §12).
    HostDeclared,
}

impl Fault {
    /// Always `true`.
    ///
    /// This exists so that stickiness is a stated property of the type rather than a
    /// convention observed by whichever code happens to hold a `Fault`. There is
    /// deliberately no clear, reset, or `try_recover` path anywhere in the crate; a
    /// reviewer encountering that absence should read this method and conclude it is the
    /// design, not an oversight. Restart from a fault is a host lifecycle event — the
    /// node is restarted and re-establishes a coherent state through the ordinary
    /// restart path (§5 invariant 5) — not a state transition the core can perform on
    /// itself, because a node that has lost track of its own durable state cannot be the
    /// authority that declares itself sound again.
    #[must_use]
    pub fn is_sticky(self) -> bool {
        match self {
            Fault::IllegalTransition
            | Fault::IndeterminatePersistence
            | Fault::QuorumObligation
            | Fault::ProgressJournalDivergence
            | Fault::HostDeclared => true,
        }
    }
}
