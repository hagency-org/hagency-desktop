//! Reservations follow the actual IO/file owner, including after observer cancellation.
use std::sync::{Arc, Mutex, LazyLock};
pub(crate) const MAX_TEMPORARY_BYTES: u64 = 16 * 1024 * 1024 * 1024;
static PIPELINES: LazyLock<Arc<ByteBudget>> = LazyLock::new(Arc::default);
pub(crate) fn pipeline_budget() -> Arc<ByteBudget> {
    PIPELINES.clone()
}
static TEMPORARY: LazyLock<Arc<ByteBudget>> = LazyLock::new(Arc::default);
pub(crate) fn temporary_budget() -> Arc<ByteBudget> {
    TEMPORARY.clone()
}

#[derive(Debug, Default)]
pub(crate) struct ByteBudget(Mutex<u64>);
impl ByteBudget {
    pub fn reserve(self: &Arc<Self>, bytes: u64, maximum: u64) -> Result<Reservation, String> {
        let mut used = self.0.lock().unwrap();
        if bytes > maximum.saturating_sub(*used) {
            return Err("Attachment storage or memory budget is full".into());
        }
        *used += bytes;
        Ok(Reservation {
            budget: self.clone(),
            bytes,
            companion: None,
        })
    }
    pub fn used(&self) -> u64 {
        *self.0.lock().unwrap()
    }
}
#[derive(Debug)]
pub(crate) struct Reservation {
    budget: Arc<ByteBudget>,
    bytes: u64,
    companion: Option<Box<Reservation>>,
}
impl Reservation {
    /// Both reservations follow the same physical file/job owner through unknown cleanup.
    pub fn with_companion(mut self, companion: Reservation) -> Self {
        assert!(self.companion.is_none() && companion.companion.is_none());
        self.companion = Some(Box::new(companion));
        self
    }
    pub fn grow(&mut self, bytes: u64, maximum: u64) -> Result<(), String> {
        let mut extra = self.budget.reserve(bytes, maximum)?;
        let mut companion_extra = self
            .companion
            .as_ref()
            .map(|companion| companion.budget.reserve(bytes, maximum))
            .transpose()?;
        self.bytes += bytes;
        extra.bytes = 0;
        if let (Some(companion), Some(extra)) = (self.companion.as_mut(), companion_extra.as_mut())
        {
            companion.bytes += bytes;
            extra.bytes = 0;
        }
        Ok(())
    }
}
impl Drop for Reservation {
    fn drop(&mut self) {
        *self.budget.0.lock().unwrap() -= self.bytes;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sharing_cannot_consume_space_reserved_for_a_file_pipeline() {
        let logical = Arc::<ByteBudget>::default();
        let physical = Arc::<ByteBudget>::default();
        let pipeline = logical.reserve(90, 100).unwrap();
        assert!(logical.reserve(11, 100).is_err());
        let envelope = logical.reserve(10, 100).unwrap();
        let mut share = physical.reserve(10, 100).unwrap().with_companion(envelope);
        assert!(share.grow(1, 100).is_err());
        assert_eq!(
            physical.used(),
            10,
            "failed coupled growth rolls back its physical claim"
        );
        assert_eq!(logical.used(), 100);
        drop(pipeline);
        share.grow(5, 100).unwrap();
        assert_eq!(physical.used(), 15);
        assert_eq!(logical.used(), 15);
        drop(share);
        assert_eq!(physical.used(), 0);
        assert_eq!(logical.used(), 0);
    }
}
