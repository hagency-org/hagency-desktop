//! Account state has a private subdirectory, but every controller connects to
//! the same installation socket. The full owner-root registry key is the route.
use std::path::Path;

pub const ACCOUNTS: &str = "hosting-accounts";

pub fn valid_key(key: &str) -> bool {
    key.len() == 64
        && key
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

pub fn endpoint(home: &Path) -> (&Path, Option<&str>) {
    if let Some(parent) = home.parent()
        && parent.file_name().is_some_and(|part| part == ACCOUNTS)
        && let Some(key) = home
            .file_name()
            .and_then(|part| part.to_str())
            .filter(|key| valid_key(key))
        && let Some(base) = parent.parent()
    {
        return (base, Some(key));
    }
    (home, None)
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutedRequest<'a, T> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_scope: Option<&'a str>,
    #[serde(flatten)]
    pub request: &'a T,
}
