use reqwest::header::{AGE, CACHE_CONTROL, DATE, HeaderMap};
use serde_json::Value;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const MAX_WINDOW_MS: u64 = 31_000;

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
        Ok(now + Duration::from_millis(remaining))
    }
}

fn remaining_ms(
    until: u64,
    local_ms: u64,
    server_second_ms: Option<u64>,
    elapsed: Duration,
) -> Option<u64> {
    let local_remaining = until.checked_sub(local_ms)?;
    let remaining = if let Some(server_second_ms) = server_second_ms {
        let window = until.checked_sub(server_second_ms)?;
        if window > MAX_WINDOW_MS {
            return None;
        }
        // HTTP Date is rounded down to seconds. Charge the whole unknown second
        // and the entire round trip, including any later identity check.
        let server_remaining = Duration::from_millis(window.checked_sub(1_000)?).checked_sub(elapsed)?;
        local_remaining.min(server_remaining.as_millis() as u64)
    } else {
        local_remaining
    };
    (1..=MAX_WINDOW_MS)
        .contains(&remaining)
        .then_some(remaining)
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(remaining_ms(164_000, 100_000, date, half_second), None);
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
