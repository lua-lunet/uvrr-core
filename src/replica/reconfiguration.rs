//! Reconfiguration (§8.7.1–§8.7.8): membership change, weight change, and the
//! non-stop pivot.
//!
//! Scope: the era model and the membership/weight fold (§8.7.1–§8.7.3), the
//! `Void`/`Init`/`INCREMENT` system operations and their slot discipline
//! (§8.7.2), the stop-the-world transition and its ordinary view change
//! (§8.7.4–§8.7.5), the non-stop `qI`/`qII` pivot (§8.7.6–§8.7.7), and the
//! era proof a cross-era message carries (§8.7.8).
//!
//! The reconfiguration path is not yet implemented. [`Input::Reconfigure`] is
//! refused with the named, tested [`PlanRejection::Unsupported`] — never a
//! silent no-op, never a placeholder handler — and the configuration fold it
//! will drive already lives in [`crate::configuration`]. The `Pivot` record
//! (§8.7.6) states the shape of a legal pivot; constructing one is this
//! path's work.
