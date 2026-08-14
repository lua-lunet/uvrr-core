use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use vrr::configuration::VOID_SLOT;
use vrr::ids::Slot;
use vrr::journal::{Journal, JournalError, JournalView, LogEntry, RangeOutcome};

/// Test-local retention control. Slot zero means no drop request because it is
/// the protocol sentinel and can never be a journal position.
#[derive(Clone, Debug)]
pub struct DropControl(Arc<AtomicU64>);

impl DropControl {
    pub fn drop_next_accept_at(&self, slot: Slot) {
        assert_ne!(slot, Slot::FIRST, "slot zero is the no-drop sentinel");
        self.0.store(slot.0, Ordering::Relaxed);
    }
}

/// A host journal that can let one accepted slot go immediately while keeping
/// its logical accepted frontier. It exists only to exercise retention paths
/// through the public `Journal` contract.
#[derive(Clone, Debug)]
pub struct HoleyLog {
    accepted: Option<Slot>,
    held: BTreeMap<Slot, LogEntry>,
    drop_on_accept: Arc<AtomicU64>,
}

impl HoleyLog {
    pub fn controlled() -> (Self, DropControl) {
        let control = Arc::new(AtomicU64::new(0));
        (
            Self {
                accepted: None,
                held: BTreeMap::new(),
                drop_on_accept: Arc::clone(&control),
            },
            DropControl(control),
        )
    }

    fn validate_append(&self, entries: &[LogEntry]) -> Result<(), JournalError> {
        let Some(_) = entries.first() else {
            return Ok(());
        };
        let mut expected = match self.accepted {
            Some(frontier) => frontier.next().ok_or(JournalError::SlotExhausted)?,
            None => VOID_SLOT,
        };
        for entry in entries {
            if entry.slot < expected {
                return Err(JournalError::SlotOccupied(entry.slot));
            }
            if entry.slot != expected {
                return Err(JournalError::NonContiguous {
                    expected,
                    got: entry.slot,
                });
            }
            expected = entry.slot.next().ok_or(JournalError::SlotExhausted)?;
        }
        Ok(())
    }
}

impl JournalView for HoleyLog {
    fn accepted(&self) -> Option<Slot> {
        self.accepted
    }

    fn get(&self, slot: Slot) -> Option<&LogEntry> {
        self.held.get(&slot)
    }

    fn iter_range(&self, from: Slot, to: Slot) -> impl DoubleEndedIterator<Item = &LogEntry> {
        self.held.range(from..=to).map(|(_, entry)| entry)
    }

    fn retained(&self) -> (Slot, Slot) {
        match (self.held.keys().next(), self.held.keys().next_back()) {
            (Some(first), Some(last)) => (*first, *last),
            _ => (VOID_SLOT, Slot::FIRST),
        }
    }

    fn copy_out(&self, from: Slot, to: Slot, into: &mut Vec<LogEntry>) -> RangeOutcome {
        if self.held.is_empty() {
            return RangeOutcome::Empty;
        }
        let (first, _) = self.retained();
        if from < first {
            return RangeOutcome::BelowRetention { slot: from, first };
        }
        into.extend(self.iter_range(from, to).cloned());
        match self.accepted {
            Some(frontier) if frontier < to => RangeOutcome::Short { through: frontier },
            _ => RangeOutcome::Complete,
        }
    }
}

impl Journal for HoleyLog {
    type View = HoleyLog;

    fn view(&self) -> Self::View {
        self.clone()
    }

    fn accept(&mut self, entries: &[LogEntry]) -> Result<(), JournalError> {
        self.validate_append(entries)?;
        let dropped = self.drop_on_accept.swap(0, Ordering::Relaxed);
        for entry in entries {
            if entry.slot.0 != dropped {
                self.held.insert(entry.slot, entry.clone());
            }
        }
        if let Some(last) = entries.last() {
            self.accepted = Some(last.slot);
        }
        Ok(())
    }

    fn install_suffix(&mut self, from: Slot, suffix: &[LogEntry]) -> Result<(), JournalError> {
        let Some(first) = suffix.first() else {
            return Err(JournalError::EmptySuffix);
        };
        if first.slot < from {
            return Err(JournalError::SlotOccupied(first.slot));
        }
        if first.slot != from {
            return Err(JournalError::NonContiguous {
                expected: from,
                got: first.slot,
            });
        }
        for pair in suffix.windows(2) {
            let expected = pair[0].slot.next().ok_or(JournalError::SlotExhausted)?;
            if expected != pair[1].slot {
                return Err(JournalError::NonContiguous {
                    expected,
                    got: pair[1].slot,
                });
            }
        }
        suffix
            .last()
            .expect("the suffix is non-empty")
            .slot
            .next()
            .ok_or(JournalError::SlotExhausted)?;
        self.held.retain(|slot, _| *slot < from);
        self.held
            .extend(suffix.iter().map(|entry| (entry.slot, entry.clone())));
        self.accepted = suffix.last().map(|entry| entry.slot);
        Ok(())
    }
}
