//! Publish SDK replacement as a synchronous native-authority boundary.
//! The retirement callback must not re-enter this slot. Callers holding another
//! publication lock may read `revision()`, but must not acquire `get()` there.
use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};

pub(crate) struct ClientSlot<T> {
    client: Mutex<Option<T>>,
    revision: AtomicU64,
}
impl<T> ClientSlot<T> {
    pub(crate) const fn new() -> Self {
        Self {
            client: Mutex::new(None),
            revision: AtomicU64::new(0),
        }
    }
    pub(crate) fn revision(&self) -> u64 {
        self.revision.load(Ordering::Acquire)
    }
    /// Advance the observable identity before retiring old native authorities.
    /// Keep the slot locked until retirement and replacement both finish, so
    /// no getter can observe the next SDK client while old authority is live.
    pub(crate) fn replace(&self, client: Option<T>, retire: impl FnOnce()) -> Option<T> {
        let mut current = self
            .client
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        self.revision
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |old| {
                old.checked_add(1)
            })
            .expect("SDK client revision exhausted");
        retire();
        std::mem::replace(&mut *current, client)
    }
}
impl<T: Clone> ClientSlot<T> {
    pub(crate) fn get(&self) -> Option<T> {
        self.client
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::{Arc, mpsc},
        thread,
        time::Duration,
    };

    #[test]
    fn sdk_revision_is_visible_before_retirement_finishes_and_get_waits() {
        let slot = Arc::new(ClientSlot::new());
        slot.replace(Some("old"), || {});
        let (entered, entering) = mpsc::sync_channel(1);
        let (release, released) = mpsc::sync_channel(1);
        let writer_slot = slot.clone();
        let writer = thread::spawn(move || {
            writer_slot.replace(Some("new"), || {
                entered.send(writer_slot.revision()).unwrap();
                released.recv().unwrap();
            })
        });
        assert_eq!(entering.recv_timeout(Duration::from_secs(2)).unwrap(), 2);
        assert_eq!(slot.revision(), 2);
        let (started, starting) = mpsc::sync_channel(1);
        let (read, received) = mpsc::sync_channel(1);
        let reader_slot = slot.clone();
        let reader = thread::spawn(move || {
            started.send(()).unwrap();
            read.send(reader_slot.get()).unwrap();
        });
        starting.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(received.recv_timeout(Duration::from_millis(50)).is_err());
        release.send(()).unwrap();
        assert_eq!(writer.join().unwrap(), Some("old"));
        assert_eq!(
            received.recv_timeout(Duration::from_secs(2)).unwrap(),
            Some("new")
        );
        reader.join().unwrap();
    }

    #[test]
    fn sdk_replacement_cannot_overtake_an_unfinished_retirement() {
        let slot = Arc::new(ClientSlot::new());
        slot.replace(Some("original"), || {});
        let (entered, entering) = mpsc::sync_channel(1);
        let (release, released) = mpsc::sync_channel(1);
        let first_slot = slot.clone();
        let first = thread::spawn(move || {
            first_slot.replace(Some("first"), || {
                entered.send(()).unwrap();
                released.recv().unwrap();
            })
        });
        entering.recv_timeout(Duration::from_secs(2)).unwrap();
        let (started, starting) = mpsc::sync_channel(1);
        let (retired, retiring) = mpsc::sync_channel(1);
        let second_slot = slot.clone();
        let second = thread::spawn(move || {
            started.send(()).unwrap();
            second_slot.replace(Some("second"), || {
                retired.send(second_slot.revision()).unwrap();
            })
        });
        starting.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(retiring.recv_timeout(Duration::from_millis(50)).is_err());
        assert_eq!(slot.revision(), 2);
        release.send(()).unwrap();
        assert_eq!(first.join().unwrap(), Some("original"));
        assert_eq!(retiring.recv_timeout(Duration::from_secs(2)).unwrap(), 3);
        assert_eq!(second.join().unwrap(), Some("first"));
        assert_eq!(slot.get(), Some("second"));
    }

    #[test]
    fn same_identity_replacement_and_removal_still_change_revision() {
        let slot = ClientSlot::new();
        assert_eq!(slot.revision(), 0);
        assert_eq!(slot.replace(Some("same"), || {}), None);
        assert_eq!(slot.revision(), 1);
        assert_eq!(slot.replace(Some("same"), || {}), Some("same"));
        assert_eq!(slot.revision(), 2);
        assert_eq!(slot.replace(None, || {}), Some("same"));
        assert_eq!(slot.revision(), 3);
        assert_eq!(slot.get(), None);
    }
}
