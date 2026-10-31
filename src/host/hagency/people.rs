//! Read-only: people on the owner's own homeserver, so the list of who may
//! request from a pool is picked from real accounts instead of typed blind.
//! A script sends only a search text. The server is the signed-in account's,
//! every result is on it, and nothing is written, invited or sent.
use super::{Host, Lease, parse};
use serde::Deserialize;
use serde_json::{Value, json};

pub(crate) const SEARCH: &str = "hagency.people.search";
const MAX_QUERY: usize = 64;
/// Matches for a search; the starting list (an empty search) may be longer.
const MAX_FOUND: usize = 20;
const MAX_KNOWN: usize = 50;
/// Bounds on the shared-room walk that builds the starting list.
const MAX_ROOMS: usize = 200;
const MAX_MEMBERS: usize = 500;

/// `{query}`: what the owner typed, or "" for the people they share rooms with.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SearchQuery {
    query: String,
}
impl SearchQuery {
    pub(crate) fn parse(value: Value) -> Result<Self, String> {
        let mut query: Self = parse(value)?;
        query.query = query.query.trim().to_owned();
        if query.query.chars().count() > MAX_QUERY || query.query.chars().any(char::is_control) {
            return Err("invalid_management_arguments".into());
        }
        Ok(query)
    }
}

/// The local part a typed name stands for on `server`: "LiSi", "@lisi" or
/// "@lisi:server" are all "lisi". New Matrix user IDs are lower case, so
/// a capital letter can only mean the lower-case name. None for another
/// server or a name that can't be a user ID.
pub(crate) fn local_part(typed: &str, server: &str) -> Option<String> {
    let typed = typed.trim().trim_start_matches('@');
    let (name, on) = typed.split_once(':').unwrap_or((typed, server));
    let name = name.to_lowercase();
    (on.eq_ignore_ascii_case(server)
        && !name.is_empty()
        && name.len() + server.len() < 255
        && name.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._=-/+".contains(&b)))
    .then_some(name)
}

/// One account, as the page lists it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Person {
    pub id: String,
    pub name: Option<String>,
}
impl Person {
    fn matches(&self, needle: &str) -> bool {
        self.id.to_lowercase().contains(needle) || self.name.as_deref().is_some_and(|n| n.to_lowercase().contains(needle))
    }
}

/// The list the page gets: accounts on `server` only, never the owner, each
/// once, directory matches first, then people the owner shares rooms with
/// whose ID or name contains the search. An empty search lists those people.
pub(crate) fn pick(found: Vec<Person>, known: &[Person], me: &str, server: &str, query: &str) -> Vec<Person> {
    let needle = query.to_lowercase();
    let on_server = |p: &Person| p.id.split_once(':').is_some_and(|(_, s)| s.eq_ignore_ascii_case(server)) && p.id != me;
    let mut out: Vec<Person> = Vec::new();
    let candidates = found.into_iter().chain(known.iter().filter(|p| needle.is_empty() || p.matches(&needle)).cloned());
    for person in candidates.filter(on_server) {
        if !out.iter().any(|p| p.id == person.id) {
            out.push(person);
        }
    }
    out.truncate(if needle.is_empty() { MAX_KNOWN } else { MAX_FOUND });
    out
}

/// People the owner shares joined rooms with, on the owner's server, sorted
/// by name. Registered Agents aren't people who request a pool: left out.
async fn known_people(client: &matrix_sdk::Client, me: &str, server: &str) -> Vec<Person> {
    let agents: Vec<String> = crate::agent_access::current()
        .agent_registry
        .agents()
        .map(|(id, _)| id.to_string())
        .collect();
    let mut out: Vec<Person> = Vec::new();
    for room in client.joined_rooms().into_iter().take(MAX_ROOMS) {
        if crate::moments::is_moments(&room) {
            continue;
        }
        let Ok(members) = room.members_no_sync(matrix_sdk::RoomMemberships::JOIN).await else {
            continue;
        };
        for member in members {
            let id = member.user_id().as_str();
            if id == me
                || !member.user_id().server_name().as_str().eq_ignore_ascii_case(server)
                || agents.iter().any(|agent| agent == id)
                || out.iter().any(|p| p.id == id)
            {
                continue;
            }
            out.push(Person { id: id.to_owned(), name: member.display_name().map(str::to_owned) });
        }
        if out.len() >= MAX_MEMBERS {
            break;
        }
    }
    out.sort_by_key(|p| (p.name.clone().unwrap_or_default().to_lowercase(), p.id.clone()));
    out
}

/// Whether an account exists on the server: its profile answers, or the
/// server says there is no such user. None when that can't be told now.
async fn exists(client: &matrix_sdk::Client, id: &str) -> Option<bool> {
    let user = matrix_sdk::ruma::UserId::parse(id).ok()?;
    match client.account().fetch_user_profile_of(&user).await {
        Ok(_) => Some(true),
        Err(matrix_sdk::Error::Http(error)) if error.as_client_api_error().is_some_and(|e| e.status_code.as_u16() == 404) => Some(false),
        Err(_) => None,
    }
}

impl Host {
    /// `hagency.people.search`: accounts on the owner's homeserver that match
    /// what was typed, or the people the owner shares rooms with for an empty
    /// search; and for a typed username, whether that account exists. Reads
    /// the signed-in client's directory and profiles only.
    pub(crate) async fn people_search(&self, lease: &Lease, args: Value) -> Result<Value, String> {
        let account = self.check(lease)?;
        lease.authorize(&account, SEARCH, None)?;
        let query = SearchQuery::parse(args)?;
        let server = account.split_once(':').map(|(_, server)| server.to_owned()).ok_or("matrix_sign_in_required")?;
        let client = crate::sliding_sync::get_client().ok_or("matrix_sign_in_required")?;
        if client.user_id().map(|id| id.as_str()) != Some(account.as_str()) {
            return Err("management_scope_changed".into());
        }
        let known = known_people(&client, &account, &server).await;
        let mut found = Vec::new();
        if !query.query.is_empty() {
            // The directory may not list everyone; the shared rooms and the exact check below cover the rest.
            if let Ok(reply) = client.search_users(&query.query, MAX_FOUND as u64).await {
                found = reply
                    .results
                    .into_iter()
                    .map(|user| Person { id: user.user_id.to_string(), name: user.display_name })
                    .collect();
            }
        }
        let people = pick(found, &known, &account, &server, &query.query);
        let exact = match local_part(&query.query, &server).filter(|_| !query.query.is_empty()) {
            Some(local) => {
                let id = format!("@{local}:{server}");
                let exists = if people.iter().any(|p| p.id == id) { Some(true) } else { exists(&client, &id).await };
                json!({"userId": id, "exists": exists})
            }
            None => Value::Null,
        };
        let account = self.check(lease)?;
        lease.authorize(&account, SEARCH, None)?;
        Ok(json!({
            "server": server,
            "query": query.query,
            "people": people.iter().map(|p| json!({"userId": p.id, "displayName": p.name})).collect::<Vec<_>>(),
            "exact": exact,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn person(id: &str, name: Option<&str>) -> Person {
        Person { id: id.into(), name: name.map(str::to_owned) }
    }
    #[test]
    fn a_search_is_one_bounded_text_and_nothing_else() {
        assert_eq!(SearchQuery::parse(json!({"query": "  LiSi "})).unwrap().query, "LiSi");
        assert_eq!(SearchQuery::parse(json!({"query": ""})).unwrap().query, "");
        for forged in [
            json!({}),
            json!({"query": 7}),
            json!({"query": "x".repeat(65)}),
            json!({"query": "a\nb"}),
            json!({"query": "lisi", "server": "elsewhere.test"}),
            json!({"query": "lisi", "limit": 500}),
            json!(["lisi"]),
        ] {
            assert_eq!(SearchQuery::parse(forged.clone()).unwrap_err(), "invalid_management_arguments", "{forged}");
        }
    }
    #[test]
    fn a_typed_name_means_its_lower_case_account_on_the_owners_server() {
        let server = "matrix.example.test:8448";
        assert_eq!(local_part("LiSi", server).as_deref(), Some("lisi"));
        assert_eq!(local_part("@zhaoliu", server).as_deref(), Some("zhaoliu"));
        assert_eq!(local_part("@zhaoliu:matrix.example.test:8448", server).as_deref(), Some("zhaoliu"));
        assert_eq!(local_part("li.si_2", server).as_deref(), Some("li.si_2"));
        for refused in ["", "@", "li si", "张三", "@zhaoliu:elsewhere.test", "zhaoliu:matrix.example.test"] {
            assert_eq!(local_part(refused, server), None, "{refused}");
        }
    }
    #[test]
    fn the_list_is_confined_to_the_owners_server_without_the_owner() {
        let server = "example.test";
        let me = "@owner:example.test";
        let known = [
            person("@lisi:example.test", Some("Li Si")),
            person("@zhaoliu:example.test", Some("Zhao Liu")),
            person("@visitor:elsewhere.test", Some("Li from elsewhere")),
        ];
        // The starting list: the people the owner shares rooms with, on this server.
        let start = pick(Vec::new(), &known, me, server, "");
        assert_eq!(start.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(), ["@lisi:example.test", "@zhaoliu:example.test"]);
        // A search: directory matches first, then shared-room people whose name or ID matches, each once.
        let found = vec![
            person("@lisi:example.test", Some("Li Si")),
            person("@liming:example.test", None),
            person("@owner:example.test", Some("Owner")),
            person("@li:elsewhere.test", None),
        ];
        let matches = pick(found, &known, me, server, "Li");
        // "Zhao Liu" is no directory match, but the owner chats with them and @zhaoliu contains "li".
        assert_eq!(matches.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(), ["@lisi:example.test", "@liming:example.test", "@zhaoliu:example.test"]);
        // Bounded.
        let many: Vec<Person> = (0..80).map(|n| person(&format!("@user{n}:example.test"), None)).collect();
        assert_eq!(pick(many.clone(), &[], me, server, "user").len(), MAX_FOUND);
        assert_eq!(pick(Vec::new(), &many, me, server, "").len(), MAX_KNOWN);
    }
    #[test]
    fn the_search_is_declared_read_only_and_grants_no_management_permission() {
        assert!(octosense_app_contract::hagency::SERVICES.contains(&SEARCH));
        assert!(octosense_app_contract::KNOWN_CAPABILITIES.contains(&SEARCH));
        assert!(octosense_app_contract::hagency::words(SEARCH).is_some());
        let shipped: Value = serde_json::from_str(include_str!("../../../apps/hagency/bundle/manifest.json")).unwrap();
        assert!(shipped["capabilities"].as_array().unwrap().iter().any(|cap| cap == SEARCH));
        let lease = rinx_miniapp_core::SessionAuthority::default().issue(
            crate::miniapps::InstanceId {
                app: octosense_app_contract::hagency::APP_ID.into(),
                account: "@owner:test".into(),
                room: None,
                generation: 1,
            },
            [SEARCH.to_owned()].into(),
            Default::default(),
            std::time::Instant::now() + std::time::Duration::from_secs(60),
        );
        assert!(super::super::permissions(&lease).is_empty());
    }
}
