use reqwest::header::{AGE, CACHE_CONTROL, DATE, HeaderMap};
use serde_json::Value;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const MAX_WINDOW_MS: u64 = 31_000;
/// The authorization window this host asks the server for when it renews its
/// proof. A server that grants no more than the default one ignores the ask.
/// It is the longest activity lease a process accepts: a lease follows the
/// window, and one constant says how long both may be.
pub(super) const REQUESTED_WINDOW_MS: u64 =
    hagency_runtime::owned::LONGEST_ACTIVITY_LEASE.as_millis() as u64;
/// The longest window of a proof from a server that granted what was asked.
const MAX_REQUESTED_WINDOW_MS: u64 = REQUESTED_WINDOW_MS + 1_000;

pub(super) struct ProofClock {
    started: Instant,
    server_second_ms: Option<u64>,
}

impl ProofClock {
    pub(super) fn new(started: Instant, headers: &HeaderMap) -> Self {
        // Only use the date on this fresh, non-storable authorization response.
        // An undated response retains the original local-clock check.
        let no_store = headers
            .get_all(CACHE_CONTROL)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .flat_map(|s| s.split(','))
            .any(|s| s.trim().eq_ignore_ascii_case("no-store"));
        let mut dates = headers.get_all(DATE).iter();
        let server_second_ms = dates
            .next()
            .filter(|_| {
                dates.next().is_none() && no_store && headers.get_all(AGE).iter().all(|v| v == "0")
            })
            .and_then(|v| httpdate::parse_http_date(v.to_str().ok()?).ok())
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .and_then(|d| u64::try_from(d.as_millis()).ok());
        Self {
            started,
            server_second_ms,
        }
    }

    pub(super) fn deadline(&self, body: &Value) -> Result<Instant, &'static str> {
        self.deadline_ms(
            body["validUntilMs"]
                .as_u64()
                .ok_or("invalid_server_response")?,
        )
    }

    pub(super) fn deadline_ms(&self, until: u64) -> Result<Instant, &'static str> {
        self.bounds_ms(until).map(|(deadline, _)| deadline)
    }

    /// The proof's deadline, and how much of its window the deadline leaves
    /// out as the local clock alone reads that window: the whole second the
    /// server's date may be behind, and the round trip. A deadline the server
    /// derives from the same window, such as an execution lease, reaches the
    /// host as server time and is read with the local clock, so it is compared
    /// with the deadline plus this allowance, not with the deadline alone.
    pub(super) fn bounds(&self, body: &Value) -> Result<(Instant, Duration), &'static str> {
        self.bounds_ms(
            body["validUntilMs"]
                .as_u64()
                .ok_or("invalid_server_response")?,
        )
    }

    fn bounds_ms(&self, until: u64) -> Result<(Instant, Duration), &'static str> {
        let now = Instant::now();
        let local_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| "invalid_local_clock")?
            .as_millis() as u64;
        let remaining = remaining_ms(
            until,
            local_ms,
            self.server_second_ms,
            now.duration_since(self.started),
        )
        .ok_or("authorization_time_window_unavailable")?;
        let bound = window_bound(until, self.server_second_ms);
        Ok((
            now + Duration::from_millis(remaining),
            Duration::from_millis(allowance_ms(until, local_ms, remaining, bound)),
        ))
    }
}

/// Where a proof's stated end falls on the local clock, for an answer whose
/// bounds could not be taken: now, when that end has passed. It only ever
/// serves to cut a proof in hand, never to extend one.
pub(super) fn local_end(body: &Value) -> Option<Instant> {
    let until = body["validUntilMs"].as_u64()?;
    let local_ms = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_millis() as u64;
    Some(Instant::now() + Duration::from_millis(until.saturating_sub(local_ms)))
}

/// The longest window the proof that ends at `until` may have. Only the
/// server's own date can show a window longer than the default one: an undated
/// proof is read with the local clock alone and keeps the default bound.
fn window_bound(until: u64, server_second_ms: Option<u64>) -> u64 {
    match server_second_ms.and_then(|second| until.checked_sub(second)) {
        Some(window) if window > MAX_WINDOW_MS => MAX_REQUESTED_WINDOW_MS,
        _ => MAX_WINDOW_MS,
    }
}

/// What `remaining` leaves out of the window that ends at `until`, as the
/// local clock reads it. Never more than `bound`, the longest window the
/// proof may have.
fn allowance_ms(until: u64, local_ms: u64, remaining: u64, bound: u64) -> u64 {
    until
        .saturating_sub(local_ms)
        .min(bound)
        .saturating_sub(remaining)
}

fn remaining_ms(
    until: u64,
    local_ms: u64,
    server_second_ms: Option<u64>,
    elapsed: Duration,
) -> Option<u64> {
    let bound = window_bound(until, server_second_ms);
    let local_remaining = until.checked_sub(local_ms)?;
    let remaining = if let Some(server_second_ms) = server_second_ms {
        let window = until.checked_sub(server_second_ms)?;
        if window > bound {
            return None;
        }
        // HTTP Date is rounded down to seconds. Charge the whole unknown second
        // and the entire round trip, including any later identity check.
        let server_remaining = Duration::from_millis(window.checked_sub(1_000)?).checked_sub(elapsed)?;
        local_remaining.min(server_remaining.as_millis() as u64)
    } else {
        local_remaining
    };
    (1..=bound).contains(&remaining).then_some(remaining)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stated_end_that_has_passed_falls_on_now() {
        let now_ms = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
        let before = Instant::now();
        let passed = local_end(&serde_json::json!({"validUntilMs": now_ms - 5_000})).unwrap();
        assert!(passed >= before && passed <= Instant::now());
        let ahead = local_end(&serde_json::json!({"validUntilMs": now_ms + 10_000})).unwrap();
        assert!(ahead > before + Duration::from_secs(9) && ahead <= Instant::now() + Duration::from_secs(10));
        assert!(local_end(&serde_json::json!({})).is_none());
    }

    #[test]
    fn fresh_dated_proof_is_bounded_by_both_clocks_and_the_request_start() {
        let date = Some(132_000);
        let half_second = Duration::from_millis(500);
        assert_eq!(
            remaining_ms(162_000, 100_000, date, half_second),
            Some(28_500)
        );
        assert_eq!(remaining_ms(162_000, 161_900, date, half_second), Some(100));
        assert_eq!(remaining_ms(162_000, 162_000, date, half_second), None);
        // A dated proof may have the window this host asks for, and no more.
        assert_eq!(
            remaining_ms(164_000, 100_000, date, half_second),
            Some(30_500)
        );
        assert_eq!(
            remaining_ms(252_000, 132_200, date, half_second),
            Some(118_500)
        );
        assert_eq!(remaining_ms(254_000, 132_200, date, half_second), None);
        // An undated proof cannot show a longer window than the default one.
        assert_eq!(remaining_ms(252_000, 132_200, None, half_second), None);
        assert_eq!(
            remaining_ms(162_000, 100_000, date, Duration::from_secs(29)),
            None
        );
        assert_eq!(remaining_ms(162_000, 100_000, None, half_second), None);
        assert_eq!(
            remaining_ms(131_000, 100_000, None, half_second),
            Some(31_000)
        );
    }

    #[test]
    fn the_allowance_is_what_the_server_date_and_the_round_trip_took() {
        let date = Some(132_000);
        let half_second = Duration::from_millis(500);
        // A local clock in step with the server, 200 ms into the server's
        // second: the proof keeps 28.5 s of a window the local clock reads as
        // 29.8 s.
        let remaining = remaining_ms(162_000, 132_200, date, half_second).unwrap();
        assert_eq!(remaining, 28_500);
        assert_eq!(allowance_ms(162_000, 132_200, remaining, MAX_WINDOW_MS), 1_300);
        // A local clock that is ahead bounds the proof by itself: nothing was
        // taken beyond what it reads.
        let remaining = remaining_ms(162_000, 140_000, date, half_second).unwrap();
        assert_eq!(remaining, 22_000);
        assert_eq!(allowance_ms(162_000, 140_000, remaining, MAX_WINDOW_MS), 0);
        // An undated proof is read with the local clock alone.
        let remaining = remaining_ms(131_000, 100_000, None, half_second).unwrap();
        assert_eq!(allowance_ms(131_000, 100_000, remaining, MAX_WINDOW_MS), 0);
        // A local clock far behind never widens the allowance past one window.
        let remaining = remaining_ms(162_000, 100_000, date, half_second).unwrap();
        assert_eq!(
            allowance_ms(162_000, 100_000, remaining, MAX_WINDOW_MS),
            31_000 - 28_500
        );
        assert_eq!(allowance_ms(162_000, 170_000, 1, MAX_WINDOW_MS), 0);
        // A proof with the longer window this host asked for keeps the same
        // allowance: the server's second and the round trip.
        assert_eq!(window_bound(162_000, date), MAX_WINDOW_MS);
        assert_eq!(window_bound(252_000, date), MAX_REQUESTED_WINDOW_MS);
        assert_eq!(window_bound(252_000, None), MAX_WINDOW_MS);
        let remaining = remaining_ms(252_000, 132_200, date, half_second).unwrap();
        assert_eq!(
            allowance_ms(252_000, 132_200, remaining, MAX_REQUESTED_WINDOW_MS),
            1_300
        );
    }

    #[test]
    fn only_an_unambiguous_non_storable_response_supplies_server_time() {
        let mut headers = HeaderMap::new();
        let started = Instant::now();
        headers.insert(DATE, "Thu, 01 Jan 1970 00:02:12 GMT".parse().unwrap());
        assert_eq!(ProofClock::new(started, &headers).server_second_ms, None);
        headers.insert(CACHE_CONTROL, "private, no-store".parse().unwrap());
        assert_eq!(
            ProofClock::new(started, &headers).server_second_ms,
            Some(132_000)
        );
        headers.insert(AGE, "2".parse().unwrap());
        assert_eq!(ProofClock::new(started, &headers).server_second_ms, None);
        headers.remove(AGE);
        headers.append(DATE, "Thu, 01 Jan 1970 00:02:13 GMT".parse().unwrap());
        assert_eq!(ProofClock::new(started, &headers).server_second_ms, None);
    }
}
