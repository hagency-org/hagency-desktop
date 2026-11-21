//! Matrix identifiers embedded in the managed Agent wire protocol.
/// Legacy Room IDs include a nonempty domain; room v12 uses a 43-character
/// unpadded URL-safe hash. Existence and authority require server observations.
pub fn valid_room_id(value: &str) -> bool {
    value.len() <= 255
        && value.starts_with('!')
        && !value.chars().any(|c| c.is_whitespace() || c.is_control())
        && (value[1..]
            .split_once(':')
            .is_some_and(|(local, server)| !local.is_empty() && !server.is_empty())
            || (value.len() == 44
                && value[1..]
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')))
}
#[cfg(test)]
mod tests {
    #[test]
    fn legacy_and_v12_room_ids_keep_strict_boundaries() {
        for room in [
            "!room:example.test",
            "!Nhcu5BS-UMnFX7hBVfVSoXiD7OgH6iRT-xyIuqDnpYQ",
        ] {
            assert!(super::valid_room_id(room));
        }
        for room in [
            "",
            "!",
            "!short",
            "!:test",
            "!room:",
            "!bad room:test",
            "!bad\n:test",
            "!Nhcu5BS+UMnFX7hBVfVSoXiD7OgH6iRT-xyIuqDnpYQ",
        ] {
            assert!(!super::valid_room_id(room), "{room}");
        }
    }
}
