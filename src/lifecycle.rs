//! The boot gate: the marker transition machine, the host's durable-write
//! boundary, and the typestate driver that owns the write schedules
//! (`docs/uvrr-boot-gate.md`).
//!
//! The crate owns the machine — which marker, which copies, when, and in
//! what order. The host owns the writes: it implements
//! [`LifecycleStore`] with its durable mechanics (a superblock quorum, or
//! plain marker files in a test) and plugs it into the driver. Every
//! schedule of `docs/uvrr-boot-gate.md` §3 is fixed by the types: a
//! transition called out of order has no type to be called on, and the
//! classification's verdict is carried in proof tokens no host can
//! construct.
//!
//! A crash is the absence of transitions — no write, no state, nothing
//! (`docs/uvrr-reincarnation.md` §1: a crash is final for the protocol
//! identity).

/// A durable node identity: the incarnation the four superblocks record.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Incarnation(pub u64);

impl Incarnation {
    /// The bumped identity: exactly one past the current one (the
    /// reincarnate branch of §5.1). Refused at exhaustion — a wrapped
    /// identity would make a superseded one indistinguishable from a
    /// current one.
    ///
    /// The band invariant (G2, `docs/architecture.md`): the bump is the sole
    /// constructor of a higher identity, and the identity it returns is
    /// strictly greater than the one it supersedes — the superseded band
    /// never re-enters circulation. The surrounding `checked_add` establishes
    /// the impossibility; the `assert!` is the tripwire, release included.
    #[must_use]
    pub fn bump(self) -> Option<Incarnation> {
        let next = self.0.checked_add(1)?;
        assert!(
            next > self.0,
            "the bumped identity must lie in a band disjoint from the identity it supersedes"
        );
        Some(Incarnation(next))
    }
}

/// One superblock copy's marker (§5.1): one state of the ordered marker
/// transition system. Each state names the transition that must have
/// completed for it to exist:
///
/// ```text
/// Running ──stop──> Stopping ──drain──> Stopped ──boot, 2-of-4──> Restarting
///                   (4x write)  flush     (4x write)              (4x write)
///                               WALs + grids
/// Running ──crash──> (markers unchanged) ──boot, no 2-of-4 Stopped──> Joining
///                                                        (bump, 4x write)
/// ```
///
/// No `Started` state is written: no safety logic looks for `Started`, it
/// looks for `Stopped` — the extra superblock write buys no safety and is
/// elided.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Marker {
    /// The stop command was received (`Running ──stop──> Stopping`, 4x).
    /// The node has stopped sending; the drain — the host flushes WALs and
    /// grids — has not yet been proven, so this state vouches for nothing.
    Stopping,
    /// The `Stopping ──drain──> Stopped` transition completed (4x). The
    /// drain happened strictly between the `Stopping` write and this one,
    /// so this state vouches for the WAL under it: there is no amnesiac
    /// risk under a `Stopped` marker.
    Stopped,
    /// The `Stopped ──boot, 2-of-4──> Restarting` transition completed
    /// (4x). The node restarted a CONTROLLED shutdown under the same
    /// identity: a member with complete state, no amnesia, ticking the
    /// full protocol — suspecting a silent primary like any backup.
    Restarting,
    /// The `crash ──boot, no 2-of-4 Stopped──> Joining` transition
    /// completed (bump, 4x). The identity was reincarnated under a bumped
    /// incarnation: NOT a member — it neither votes nor drives view
    /// change — until the forced sequence seats it.
    Joining,
}

/// One superblock copy: the identity it records and its marker.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CopyState {
    /// The identity this copy vouches for.
    pub identity: Incarnation,
    /// The copy's marker.
    pub marker: Marker,
}

/// The four superblock copies a restart reads (§2). Exactly four, as the
/// vendored TigerBeetle store's `superblock_copies`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SuperblockCopies {
    /// The four copies, in their on-disk order.
    pub copies: [CopyState; 4],
}

/// The quorum read's verdict (§5.1): did the Stopping→Stopped transition
/// complete?
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RestartClass {
    /// 2-of-4 copies hold `Stopped`: the transition completed, the drain
    /// is proven, there is no amnesiac risk — the clean stop.
    Stopped,
    /// No stopped quorum — a crash, a torn marker set, or death mid-join:
    /// no controlled shutdown. This identity is dead.
    NotStopped,
}

/// Why a restart refused (§5.1; the refusals of the Zig twin's `open`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RestartRefusal {
    /// No identity cohort reaches the open threshold — 2 of 4: the marker
    /// set is torn beyond the quorum read (`QuorumLost` in the twin).
    QuorumLost,
    /// The bump would wrap the identity space; the identity that could not
    /// be bumped is carried. A wrapped identity would make a superseded
    /// one indistinguishable from a current one, so the bump refuses
    /// instead.
    Exhausted(Incarnation),
}

/// What a restart decided (§5.1's decision table).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RestartDecision {
    /// A stopped quorum: continue under the same identity — a member with
    /// complete state, no amnesia — and write `Restarting` to all four
    /// copies (the boot of a controlled shutdown).
    Continue {
        /// The identity the node continues under.
        identity: Incarnation,
    },
    /// No stopped quorum: the identity is dead. Bump it — exactly one
    /// past the quorum-resolved identity — and write `Joining` to all
    /// four copies. The pair IS the commitment (§4): the wire phase
    /// always follows.
    Bump {
        /// The superseded identity.
        old: Incarnation,
        /// The bumped identity.
        new: Incarnation,
    },
}

impl SuperblockCopies {
    /// The quorum read (§5.1; the Zig twin's `open`, `zig/uvrr/store.zig`
    /// lines 116–147). The boot question is *did the transition complete?*,
    /// answered by 2-of-4 copies holding the state to the right of the
    /// transition — the twin's open threshold.
    ///
    /// The working quorum resolves the identity: the copies agreeing on one
    /// identity form a cohort; a cohort reaching the open threshold (2 of
    /// 4) is a working cohort; the winner is the HIGHEST-identity working
    /// cohort (higher-identity-wins INSIDE the working quorum — never the
    /// highest identity observed across all copies, which a lone stale or
    /// superseded copy cannot impose). The verdict reads the winner's
    /// cohort: [`RestartClass::Stopped`] ⟺ it holds ≥2 `Stopped` copies;
    /// anything else is not stopped.
    ///
    /// Over the marker system's written states (four uniform 4x writes per
    /// transition) the winner cohort is all four copies, so the read is
    /// exactly "2-of-4 copies hold `Stopped`"; the cohort rule is what the
    /// torn cases need to keep the twin's quorum structure (which orders
    /// by sequence where this simplified twin, without the sequence
    /// hash-chain, orders by identity).
    ///
    /// Returns `None` when no cohort reaches the threshold — the torn
    /// marker set with no quorum (the twin's `QuorumLost`).
    #[must_use]
    pub fn classify(&self) -> Option<(RestartClass, Incarnation)> {
        // Per identity: how many copies carry it, and how many of those
        // hold `Stopped` (the state right of the Stopping→Stopped
        // transition).
        let mut cohorts: Vec<(Incarnation, usize, usize)> = Vec::new();
        for copy in &self.copies {
            match cohorts
                .iter_mut()
                .find(|(identity, _, _)| *identity == copy.identity)
            {
                Some((_, count, stopped)) => {
                    *count += 1;
                    *stopped += usize::from(copy.marker == Marker::Stopped);
                }
                None => cohorts.push((
                    copy.identity,
                    1,
                    usize::from(copy.marker == Marker::Stopped),
                )),
            }
        }
        let (identity, _, stopped) = cohorts
            .into_iter()
            .filter(|&(_, count, _)| count >= 2)
            .max_by_key(|&(identity, _, _)| identity)?;
        let class = if stopped >= 2 {
            RestartClass::Stopped
        } else {
            RestartClass::NotStopped
        };
        Some((class, identity))
    }

    /// A restart: the quorum read, the decision, and the 4x marker write
    /// the decision leaves on disk (§5.1's decision table).
    ///
    /// * Stopped quorum → [`RestartDecision::Continue`]; the copies are
    ///   written `(identity, Restarting)` 4x — the boot of a controlled
    ///   shutdown. The node is a member with complete state: it ticks the
    ///   full protocol.
    /// * No stopped quorum → [`RestartDecision::Bump`]; the copies are
    ///   written `(new, Joining)` 4x — the reincarnation. The bumped
    ///   identity is one past the quorum-resolved identity, checked: an
    ///   identity one bump from `u64` exhaustion refuses rather than
    ///   wraps.
    ///
    /// The uniform 4x write is the repair: every copy that disagreed with
    /// the working quorum is rewritten from the decision — 4-of-4,
    /// satisfying the twin's repair-to-≥3-of-4 behaviour. A node dying
    /// mid-join reads no stopped quorum and reincarnates again — the table
    /// makes that fall out.
    ///
    /// # Errors
    ///
    /// [`RestartRefusal::QuorumLost`] when no identity cohort reaches the
    /// open threshold; [`RestartRefusal::Exhausted`] when the bump would
    /// wrap the identity space.
    pub fn restart(&self) -> Result<(RestartDecision, SuperblockCopies), RestartRefusal> {
        let (class, identity) = self.classify().ok_or(RestartRefusal::QuorumLost)?;
        match class {
            RestartClass::Stopped => Ok((
                RestartDecision::Continue { identity },
                self.rewrite(identity, Marker::Restarting),
            )),
            RestartClass::NotStopped => {
                let new = identity.bump().ok_or(RestartRefusal::Exhausted(identity))?;
                Ok((
                    RestartDecision::Bump { old: identity, new },
                    self.rewrite(new, Marker::Joining),
                ))
            }
        }
    }

    /// The stop command (§5.1): `Running ──stop──> Stopping`, written to
    /// all four copies. The node has stopped sending — no disk flush sits
    /// on the protocol's hot path; the drain, flushes of WALs and grids,
    /// is the HOST's and sits strictly between this write and
    /// [`SuperblockCopies::finish_stop`].
    #[must_use]
    pub fn begin_stop(&self) -> SuperblockCopies {
        self.rewrite(self.read_identity(), Marker::Stopping)
    }

    /// The drain's proof (§5.1): `Stopping ──drain──> Stopped`, written to
    /// all four copies — callable only AFTER the host drain completed. The
    /// marker order IS the drain's proof: the drain happened strictly
    /// between the `Stopping` write and this one, so a `Stopped` copy
    /// vouches for the WAL under it, and 2-of-4 `Stopped` at boot proves
    /// the clean stop with no amnesiac risk. A stop that dies partway
    /// still reads clean on the surviving quorum, correctly: the flush had
    /// already completed before the first `Stopped` write.
    #[must_use]
    pub fn finish_stop(&self) -> SuperblockCopies {
        self.rewrite(self.read_identity(), Marker::Stopped)
    }

    /// The identity the node's own copies record: the highest of the four
    /// (§2's higher-identity-wins read rule). The live node's copies are
    /// its own uniform 4x writes — the identity was resolved at boot by
    /// the quorum read ([`SuperblockCopies::classify`]); this is that
    /// identity's live read, not the boot resolution.
    fn read_identity(&self) -> Incarnation {
        self.copies
            .iter()
            .map(|copy| copy.identity)
            .max()
            .expect("four copies are always present")
    }

    /// All four copies rewritten with `identity` and `marker`. The
    /// marker writes are durable-on-write (flushed): the host applies the
    /// returned state with its sync path, which is the only disk traffic
    /// outside the stop path.
    fn rewrite(&self, identity: Incarnation, marker: Marker) -> SuperblockCopies {
        SuperblockCopies {
            copies: self.copies.map(|_| CopyState { identity, marker }),
        }
    }
}

// ---------------------------------------------------------------------------
// Debug for the session types: the store is opaque to the machine, so the
// sessions print their shape, never their store.
// ---------------------------------------------------------------------------

macro_rules! session_debug {
    ($($t:ident),* $(,)?) => {$(
        impl<S> std::fmt::Debug for $t<S> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(stringify!($t))
            }
        }
    )*};
}

session_debug!(First, Clean, Crashed, Running, Halting, Draining, Halted);

// ---------------------------------------------------------------------------
// The host boundary
// ---------------------------------------------------------------------------

/// The host's durable mechanics for the boot gate — everything the crate
/// cannot do itself and everything the host must NOT decide itself
/// (`docs/uvrr-boot-gate.md` §6).
///
/// The host implements exactly three operations; the driver owns which
/// marker is written, to which copies, and in what order. A superblock
/// quorum (four copies, checksummed, quorum-read) meets the contract
/// (`docs/vrr-durability-model.md` §5.1); plain marker files meet it in a
/// test harness.
pub trait LifecycleStore {
    /// The store's failure type.
    type Error;

    /// The quorum read: the working set of copies as the store observed
    /// them (`docs/uvrr-boot-gate.md` §2). `None` when no marker has ever
    /// been written — the first life, which has no durable identity yet.
    ///
    /// The read takes the MINIMUM progress across the working copies: a
    /// crash during marker writes must never read as a clean stop
    /// (`uvrr-termination-obligations.md` §3).
    fn read_copies(&mut self) -> Result<Option<SuperblockCopies>, Self::Error>;

    /// The forced write of a decided rewrite: every copy, durably —
    /// flushed before the call returns, because the marker vouches for
    /// what the drain has already put beneath it (`docs/uvrr-boot-gate.md`
    /// §3).
    fn commit(&mut self, copies: &SuperblockCopies) -> Result<(), Self::Error>;

    /// The drain: the host forces its WALs and grids to stable storage.
    /// The driver calls this strictly between the two halt rounds — the
    /// `Stopped` marker is written only over a proven drain
    /// (`uvrr-termination-obligations.md` §1).
    fn drain(&mut self) -> Result<(), Self::Error>;
}

// ---------------------------------------------------------------------------
// The proof tokens
// ---------------------------------------------------------------------------

/// Proof that a quorum read vouched the previous process's drain: a
/// stopped quorum was read and the clean start's latch was written
/// (`docs/uvrr-boot-gate.md` §6). Minted only by [`Clean::latch`].
///
/// The carried identity is the quorum-resolved one, for the host's
/// bookkeeping; the engine does not interpret it (the durable identity is
/// a host-side concept). The token's guarantee is the classification
/// itself: no `Vouched`, no same-identity resume.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Vouched(Incarnation);

impl Vouched {
    /// The quorum-resolved identity the clean start vouches for — the
    /// identity the resume may take.
    #[must_use]
    pub fn identity(&self) -> Incarnation {
        self.0
    }
}

/// The crashed classification's replacement pair: the superseded identity
/// and its bump (`docs/uvrr-reincarnation.md` §4 — the pair IS the
/// commitment; the wire phase always follows). Minted only by
/// [`Crashed::pair`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Bumped {
    /// The superseded identity.
    pub old: Incarnation,
    /// The bumped identity.
    pub new: Incarnation,
}

/// Proof that the engine itself observed the reincarnated node seated:
/// `Normal` at voting weight (`docs/uvrr-boot-gate.md` §6 — the deferred
/// latch's witness). Minted only by
/// [`Replica::rejoined`](crate::replica::Replica::rejoined); no host can
/// construct it, so the dirty path's latch is unreachable until the
/// rejoin has actually happened.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rejoined(());

impl Rejoined {
    /// The one minter: the replica's seated observation.
    pub(crate) fn mint() -> Rejoined {
        Rejoined(())
    }
}

// ---------------------------------------------------------------------------
// The typestate driver
// ---------------------------------------------------------------------------

/// Why [`boot`] refused.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BootError<E> {
    /// The store refused: the read or the latch write failed. The store
    /// is handed back to the caller with the error.
    Store(E),
    /// The marker set is torn beyond the quorum read — no identity cohort
    /// reaches the open threshold. A host that cannot classify from
    /// durable state is unsafe (`docs/uvrr-boot-gate.md`): it must refuse
    /// to start.
    QuorumLost,
    /// The bump would wrap the identity space; the identity that could
    /// not be bumped is carried.
    Exhausted(Incarnation),
}

/// What the boot read decided (`docs/uvrr-boot-gate.md` §1): the
/// classification chooses the path.
pub enum BootOutcome<S> {
    /// No marker has ever been written: the first life, with no durable
    /// identity yet. The host names its genesis incarnation and latches.
    First(First<S>),
    /// A stopped quorum: the previous process reached its drain point.
    /// The clean start — one latch round, then the same-identity resume.
    Clean(Clean<S>),
    /// No stopped quorum: the previous process crashed; the identity is
    /// dead. The reincarnation — the pair is decided here, the latch
    /// defers to the seated witness.
    Crashed(Crashed<S>),
}

/// The first life: no durable identity yet. The host names its genesis
/// incarnation; the latch anchors it.
pub struct First<S> {
    store: S,
}

/// The first latch's failure: the session back with the store's error.
pub type FirstLatchFailure<S> = (First<S>, <S as LifecycleStore>::Error);

/// The clean latch's failure: the session back with the store's error.
pub type CleanLatchFailure<S> = (Clean<S>, <S as LifecycleStore>::Error);

/// The halt's round-one failure: the running session back with the
/// store's error.
pub type HaltFailure<S> = (Running<S>, <S as LifecycleStore>::Error);

/// The deferred latch's failure: the crashed session back with the boot
/// error.
pub type DeferredLatchFailure<S> = (Crashed<S>, BootError<<S as LifecycleStore>::Error>);

impl<S: LifecycleStore> First<S> {
    /// The session's store, read-only: the runtime stages what the
    /// anchor will write beneath the markers.
    pub fn store(&self) -> &S {
        &self.store
    }

    /// The latch: `(identity, Joining)` written 4x — the anchor the next
    /// boot reads. A first life has nothing to vouch for, so nothing is
    /// checked; the write exists so that a crash later reads as a crash.
    ///
    /// # Errors
    ///
    /// [`BootError::Store`] hands the session back with the store error.
    pub fn latch(mut self, identity: Incarnation) -> Result<Running<S>, FirstLatchFailure<S>> {
        let copies = SuperblockCopies {
            copies: [CopyState {
                identity,
                marker: Marker::Joining,
            }; 4],
        };
        match self.store.commit(&copies) {
            Ok(()) => Ok(Running {
                store: self.store,
                identity,
            }),
            Err(error) => Err((self, error)),
        }
    }
}

/// The clean start: the drain was vouched. One latch round, before the
/// first message, then the same-identity resume.
pub struct Clean<S> {
    store: S,
    identity: Incarnation,
}

impl<S: LifecycleStore> Clean<S> {
    /// The session's store, read-only: the runtime stages what the
    /// latch will write beneath the markers.
    pub fn store(&self) -> &S {
        &self.store
    }

    /// The quorum-resolved identity the clean start vouches for.
    #[must_use]
    pub fn identity(&self) -> Incarnation {
        self.identity
    }

    /// The latch: `(identity, Restarting)` written 4x — the boot-gate
    /// latch, before the first message is processed, so that a later
    /// crash can never be mistaken for this clean start
    /// (`docs/uvrr-boot-gate.md` §3). Returns the running session and
    /// the [`Vouched`] token that [`Replica::resume`] requires: the only
    /// same-identity constructor.
    ///
    /// # Errors
    ///
    /// [`BootError::Store`] hands the session back with the store error.
    pub fn latch(mut self) -> Result<(Running<S>, Vouched), CleanLatchFailure<S>> {
        let copies = SuperblockCopies {
            copies: [CopyState {
                identity: self.identity,
                marker: Marker::Restarting,
            }; 4],
        };
        match self.store.commit(&copies) {
            Ok(()) => Ok((
                Running {
                    store: self.store,
                    identity: self.identity,
                },
                Vouched(self.identity),
            )),
            Err(error) => Err((self, error)),
        }
    }
}

/// The crashed classification: the identity is dead. The replacement pair
/// is decided here; the durable latch defers to the seated witness.
pub struct Crashed<S> {
    store: S,
    identity: Incarnation,
}

impl<S: LifecycleStore> Crashed<S> {
    /// The session's store, read-only: the runtime stages what the
    /// deferred latch will write beneath the markers (its own store
    /// type's staging mechanism) without gaining a write path around
    /// the machine.
    pub fn store(&self) -> &S {
        &self.store
    }

    /// The replacement pair: the superseded identity and its bump, one
    /// past the quorum-resolved identity (`docs/uvrr-reincarnation.md`
    /// §4). The pair IS the commitment — the wire announcement carries
    /// it — and it is idempotent: a crash between this decision and the
    /// latch re-reads the old markers and re-decides the same pair, so
    /// the replay is absorbed.
    ///
    /// # Errors
    ///
    /// [`RestartRefusal::Exhausted`] when the bump would wrap the
    /// identity space.
    pub fn pair(&self) -> Result<Bumped, RestartRefusal> {
        let new = self
            .identity
            .bump()
            .ok_or(RestartRefusal::Exhausted(self.identity))?;
        Ok(Bumped {
            old: self.identity,
            new,
        })
    }

    /// The deferred latch: `(new, Joining)` written 4x — only after the
    /// engine's seated observation ([`Replica::rejoined`]) mints the
    /// [`Rejoined`] witness. The flush is never paid at the boundary of
    /// an uninitialised start (`docs/uvrr-boot-gate.md` §3); a crash
    /// before this write re-reads the old markers, re-decides the same
    /// pair, and replays — the deferral is safe by construction.
    ///
    /// # Errors
    ///
    /// [`BootError::Store`] hands the session back with the store error.
    pub fn latch(mut self, _witness: Rejoined) -> Result<Running<S>, DeferredLatchFailure<S>> {
        let new = match self.identity.bump() {
            Some(new) => new,
            None => {
                let exhausted = self.identity;
                return Err((self, BootError::Exhausted(exhausted)));
            }
        };
        let copies = SuperblockCopies {
            copies: [CopyState {
                identity: new,
                marker: Marker::Joining,
            }; 4],
        };
        match self.store.commit(&copies) {
            Ok(()) => Ok(Running {
                store: self.store,
                identity: new,
            }),
            Err(error) => Err((self, BootError::Store(error))),
        }
    }
}

/// A running process's session with its marker store: created by a latch
/// at boot, consumed by [`Running::begin_stop`] at halt. The store rides
/// inside — the runtime reaches it with [`Running::store_mut`] for its own
/// durable writes between the marker transitions.
pub struct Running<S> {
    store: S,
    identity: Incarnation,
}

impl<S: LifecycleStore> Running<S> {
    /// The runtime's reach into its store: between the latch and the
    /// halt, the host's own durable writes (progress, journals) route
    /// here. The marker machine itself touches the store only at its
    /// transitions.
    pub fn store_mut(&mut self) -> &mut S {
        &mut self.store
    }

    /// The running identity.
    pub fn identity(&self) -> Incarnation {
        self.identity
    }

    /// Round one of the controlled halt: `(identity, Stopping)` written
    /// 4x. The node has stopped sending; the drain — the host's — sits
    /// strictly between this write and [`Draining::finish_stop`]
    /// (`docs/uvrr-boot-gate.md` §3).
    ///
    /// # Errors
    ///
    /// The store refused; the running session is handed back with the
    /// error.
    pub fn begin_stop(mut self) -> Result<Halting<S>, HaltFailure<S>> {
        let copies = SuperblockCopies {
            copies: [CopyState {
                identity: self.identity,
                marker: Marker::Stopping,
            }; 4],
        };
        match self.store.commit(&copies) {
            Ok(()) => Ok(Halting {
                store: self.store,
                identity: self.identity,
            }),
            Err(error) => Err((self, error)),
        }
    }
}

/// The halt has begun: round one is durable. The drain is next — nothing
/// else is legal.
pub struct Halting<S> {
    store: S,
    identity: Incarnation,
}

impl<S: LifecycleStore> Halting<S> {
    /// The drain: the host forces its WALs and grids to stable storage.
    /// The `Stopped` marker of [`Draining::finish_stop`] vouches for
    /// exactly this, so the order is not a recommendation — it is the
    /// only type path to the flushed state.
    ///
    /// # Errors
    ///
    /// The store refused; the halting session is handed back with the
    /// error.
    pub fn drain(mut self) -> Result<Draining<S>, (Halting<S>, S::Error)> {
        match self.store.drain() {
            Ok(()) => Ok(Draining {
                store: self.store,
                identity: self.identity,
            }),
            Err(error) => Err((self, error)),
        }
    }
}

/// The drain completed: the flushed marker can now be written over it.
pub struct Draining<S> {
    store: S,
    identity: Incarnation,
}

impl<S: LifecycleStore> Draining<S> {
    /// Round two of the controlled halt: `(identity, Stopped)` written
    /// 4x — the drain's proof. The process may exit; the next boot's
    /// quorum read vouches for everything beneath this marker.
    ///
    /// # Errors
    ///
    /// The store refused; the draining session is handed back with the
    /// error.
    pub fn finish_stop(mut self) -> Result<Halted<S>, (Draining<S>, S::Error)> {
        let copies = SuperblockCopies {
            copies: [CopyState {
                identity: self.identity,
                marker: Marker::Stopped,
            }; 4],
        };
        match self.store.commit(&copies) {
            Ok(()) => Ok(Halted {
                store: self.store,
                identity: self.identity,
            }),
            Err(error) => Err((self, error)),
        }
    }
}

/// The controlled halt is complete: both rounds durable, the drain
/// proven. The store is handed back — the process exits.
pub struct Halted<S> {
    store: S,
    identity: Incarnation,
}

impl<S> Halted<S> {
    /// The store, for inspection or teardown.
    pub fn into_store(self) -> S {
        self.store
    }

    /// The halted identity.
    pub fn identity(&self) -> Incarnation {
        self.identity
    }
}

/// The boot gate: read the durable markers, classify the start, and hand
/// back the session whose type fixes the schedule
/// (`docs/uvrr-boot-gate.md` §1, §3).
///
/// * `None` read → [`BootOutcome::First`]: no durable identity yet.
/// * Stopped quorum → [`BootOutcome::Clean`]: the drain was vouched.
/// * No stopped quorum → [`BootOutcome::Crashed`]: the identity is dead;
///   there is no function from here to a same-identity replica.
///
/// # Errors
///
/// [`BootError::QuorumLost`] when the marker set is torn beyond the
/// quorum read — a host that cannot classify from durable state is
/// unsafe and must refuse to start; [`BootError::Store`] when the read
/// itself refused, handing the store back.
pub fn boot<S: LifecycleStore>(mut store: S) -> Result<BootOutcome<S>, (S, BootError<S::Error>)> {
    let copies = match store.read_copies() {
        Ok(copies) => copies,
        Err(error) => return Err((store, BootError::Store(error))),
    };
    let Some(copies) = copies else {
        return Ok(BootOutcome::First(First { store }));
    };
    let (class, identity) = match copies.classify() {
        Some(verdict) => verdict,
        None => return Err((store, BootError::QuorumLost)),
    };
    Ok(match class {
        RestartClass::Stopped => BootOutcome::Clean(Clean { store, identity }),
        RestartClass::NotStopped => BootOutcome::Crashed(Crashed { store, identity }),
    })
}
