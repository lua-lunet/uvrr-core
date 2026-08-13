//! State transfer: bringing a lagging, recovering or newly promoted replica current.
//!
//! Spec §4 (unavailable history), §11 (application boundary), §13.1 (bounded view-change
//! suffix). Decision W5.
//!
//! Transfer state is process-local and explicitly scoped (spec §15): it is evidence
//! gathered during one attempt, never protocol-visible state, so an abandoned transfer
//! leaves nothing behind that a later view could mistake for history.
//!
//! The core owns sequencing and completeness; the host owns sizing. There is no
//! `MAX_DATAGRAM` and no chunk-size constant here (decision W5). The core reports how
//! much it has and what it still needs, and the host decides how many bytes travel per
//! datagram — because a host that must fit a chunk into its own framing cannot be served
//! by a constant this crate guessed at compile time.
//!
//! When requested history is locally unavailable the host says so (§4). The core does not
//! ask why. It obtains an adequate state from another replica, or defers to a
//! host-specific application-state transfer facility, and a zero-weight member must
//! become adequately current here before a later committed `INCREMENT` grants it voting
//! authority.
