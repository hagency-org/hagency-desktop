//! This device's time zone, named one way for the mini app's pages and the
//! native confirmation windows, so a time on the pool's page and the same time
//! in a window read alike: 「11 月 1 日 08:00（北京时间）」.
use serde_json::{Value, json};

/// IANA names of China's one time zone, said 「北京时间」 while at UTC+8.
const BEIJING: [&str; 5] = ["Asia/Shanghai", "Asia/Chongqing", "Asia/Chungking", "Asia/Harbin", "PRC"];

/// The zone's name: `TZ` when set (as chrono's local clock reads it), else the system's.
fn zone_name() -> Option<String> {
    std::env::var("TZ")
        .ok()
        .map(|tz| tz.trim_start_matches(':').trim().to_owned())
        .filter(|tz| !tz.is_empty())
        .or_else(|| iana_time_zone::get_timezone().ok())
}
/// "UTC", "UTC+8", "UTC-5", "UTC+5:30" for an offset in minutes east of UTC.
fn utc_offset(minutes: i32) -> String {
    if minutes == 0 {
        return "UTC".into();
    }
    let sign = if minutes < 0 { '-' } else { '+' };
    let (hours, rest) = (minutes.unsigned_abs() / 60, minutes.unsigned_abs() % 60);
    if rest == 0 { format!("UTC{sign}{hours}") } else { format!("UTC{sign}{hours}:{rest:02}") }
}
/// The (English, Chinese) label of a zone at an offset: Beijing time for
/// China's zone at UTC+8, otherwise the offset, which reads the same in both.
pub(crate) fn label_for(zone: Option<&str>, offset_minutes: i32) -> (String, String) {
    if offset_minutes == 480 && zone.is_some_and(|zone| BEIJING.contains(&zone)) {
        return ("Beijing time".into(), "北京时间".into());
    }
    let offset = utc_offset(offset_minutes);
    (offset.clone(), offset)
}
/// Minutes east of UTC on this device's clock at `ms`, or now.
pub(crate) fn offset_minutes(ms: Option<u64>) -> i32 {
    use chrono::TimeZone;
    let at = ms
        .and_then(|ms| i64::try_from(ms).ok())
        .and_then(|ms| chrono::Local.timestamp_millis_opt(ms).single())
        .unwrap_or_else(chrono::Local::now);
    at.offset().local_minus_utc() / 60
}
/// The zone's label at `ms` (or now) in the owner's language.
pub(crate) fn label(ms: Option<u64>) -> String {
    let (en, zh) = label_for(zone_name().as_deref(), offset_minutes(ms));
    if crate::i18n::language() == crate::i18n::Language::Chinese { zh } else { en }
}
/// `hagency.session` gains the device's offset and the zone's label in both
/// languages, so the page can show its times as the windows do. Display
/// only: nothing here is sent anywhere or authorizes anything.
pub(super) fn display(service: &str, mut value: Value) -> Value {
    if service != "hagency.session" {
        return value;
    }
    let offset = offset_minutes(None);
    let (en, zh) = label_for(zone_name().as_deref(), offset);
    if let Some(object) = value.as_object_mut() {
        object.insert("localUtcOffsetMinutes".into(), json!(offset));
        object.insert("localZoneLabel".into(), json!({"en": en, "zh": zh}));
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn china_reads_as_beijing_time_and_everywhere_else_as_an_offset() {
        for zone in BEIJING {
            assert_eq!(label_for(Some(zone), 480), ("Beijing time".to_owned(), "北京时间".to_owned()));
        }
        // The same offset elsewhere, or China's name at another offset, is just the offset.
        assert_eq!(label_for(Some("Asia/Singapore"), 480), ("UTC+8".to_owned(), "UTC+8".to_owned()));
        assert_eq!(label_for(None, 480).1, "UTC+8");
        assert_eq!(label_for(Some("Asia/Shanghai"), 540).1, "UTC+9");
        assert_eq!(label_for(Some("UTC"), 0).0, "UTC");
        assert_eq!(label_for(Some("America/New_York"), -300).1, "UTC-5");
        assert_eq!(label_for(Some("Asia/Kolkata"), 330).0, "UTC+5:30");
        assert_eq!(label_for(Some("America/St_Johns"), -150).0, "UTC-2:30");
        assert_eq!(label_for(Some("Asia/Kathmandu"), 345).1, "UTC+5:45");
    }
    #[test]
    fn only_the_session_reply_gains_the_local_zone() {
        let session = display("hagency.session", json!({"protocol": "hagency-management-v1", "account": "@owner:test"}));
        assert_eq!(session["protocol"], "hagency-management-v1");
        let offset = session["localUtcOffsetMinutes"].as_i64().expect("offset");
        assert!((-14 * 60..=14 * 60).contains(&offset));
        assert_eq!(offset, i64::from(offset_minutes(None)));
        let label = &session["localZoneLabel"];
        assert!(label["en"].as_str().is_some_and(|en| !en.is_empty()));
        assert!(label["zh"].as_str().is_some_and(|zh| !zh.is_empty()));
        for service in ["hagency.alerts.list", "hagency.resources.get", "hagency.grants.applicants"] {
            let other = display(service, json!({"a": 1}));
            assert_eq!(other, json!({"a": 1}), "{service}");
        }
    }
}
