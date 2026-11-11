//! One due-time authority: redraws/rebinding cannot bypass a failed-read backoff.
use crate::hagency::permission_failure::Recovery;
use std::time::{Duration, Instant, SystemTime};

#[derive(Default)]
pub(super) struct PermissionRetry {
    started: bool,
    failures: u8,
    pub next: Option<Instant>,
    pub last_success: Option<SystemTime>,
}

impl PermissionRetry {
    pub fn due(&self, now: Instant) -> bool {
        !self.started || self.next.is_some_and(|at| at <= now)
    }
    pub fn begin(&mut self) {
        self.started = true;
        self.next = None;
    }
    pub fn success(&mut self, now: Instant) {
        self.failures = 0;
        self.last_success = Some(SystemTime::now());
        self.next = Some(now + Duration::from_secs(2));
    }
    pub fn failed(&mut self, recovery: Recovery, now: Instant, jitter: f64) {
        self.started = true;
        self.failures = self.failures.saturating_add(1);
        self.next = (recovery == Recovery::Transient).then(|| {
            let base = (5u64 << self.failures.saturating_sub(1).min(4)).min(60);
            now + Duration::from_secs_f64(
                (base as f64 * (0.8 + jitter.clamp(0.0, 1.0) * 0.4)).min(60.0),
            )
        });
    }
    pub fn interrupted(&mut self, now: Instant) {
        self.next = Some(now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn temporary_failures_back_off_with_bounded_jitter_and_reset_after_success() {
        let start = Instant::now();
        let mut retry = PermissionRetry::default();
        assert!(retry.due(start));
        for expected in [5.0, 10.0, 20.0, 40.0, 60.0, 60.0] {
            retry.begin();
            assert!(!retry.due(start));
            retry.failed(Recovery::Transient, start, 0.5);
            let at = retry.next.unwrap();
            assert_eq!(at.duration_since(start).as_secs_f64(), expected);
            assert!(!retry.due(at - Duration::from_millis(1)));
            assert!(retry.due(at));
        }
        retry.success(start);
        assert!(retry.last_success.is_some());
        retry.begin();
        retry.failed(Recovery::Transient, start, 0.0);
        assert_eq!(
            retry.next.unwrap().duration_since(start),
            Duration::from_secs(4)
        );
        retry.begin();
        retry.failed(Recovery::Transient, start, 1.0);
        assert_eq!(
            retry.next.unwrap().duration_since(start),
            Duration::from_secs(12)
        );
    }
    #[test]
    fn a_failed_hour_cannot_repeat_the_old_two_second_polling_loop() {
        let start = Instant::now();
        for jitter in [0.0, 0.5, 1.0] {
            let mut retry = PermissionRetry::default();
            let mut requests = 0;
            for second in 0..3600 {
                let now = start + Duration::from_secs(second);
                // Frequent render/rebind checks cannot move the original deadline.
                for _ in 0..10 {
                    if retry.due(now) {
                        requests += 1;
                        retry.begin();
                        retry.failed(Recovery::Transient, now, jitter);
                    }
                }
            }
            assert!(
                (60..85).contains(&requests),
                "jitter={jitter}, requests={requests}"
            );
        }
    }

    #[test]
    fn permanent_failures_stay_paused_until_an_explicit_check_or_new_scope() {
        let now = Instant::now();
        for recovery in [
            Recovery::SignIn,
            Recovery::Access,
            Recovery::Service,
            Recovery::Unsupported,
            Recovery::Retired,
            Recovery::Manual,
        ] {
            let mut retry = PermissionRetry::default();
            retry.begin();
            retry.failed(recovery, now, 0.5);
            assert!(retry.next.is_none());
            assert!(!retry.due(now + Duration::from_secs(86400)));
            // Manual refresh calls begin directly; only its successful result resumes polling.
            retry.begin();
            retry.success(now);
            assert!(!retry.due(now + Duration::from_secs(1)));
            assert!(retry.due(now + Duration::from_secs(2)));
        }
    }
}
