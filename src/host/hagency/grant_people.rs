//! Account-bound, read-only Matrix lookups for the native pool whitelist.
//! Directory visibility stays with the homeserver. An exact local username
//! may also be checked through its profile; a well-formed ID is not evidence.
use matrix_sdk::Client;
use ruma::{OwnedUserId, UserId, api::error::ErrorKind};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Person {
    pub id: String,
    pub display_name: Option<String>,
}

impl Person {
    pub fn label(&self) -> String {
        match self
            .display_name
            .as_deref()
            .filter(|name| !name.trim().is_empty())
        {
            Some(name) => format!("{name} ({})", self.id),
            None => self.id.clone(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct People {
    pub items: Vec<Person>,
    pub limited: bool,
}

pub(super) fn local_person(owner: &str, id: &str) -> Option<OwnedUserId> {
    let owner = UserId::parse(owner).ok()?;
    let id = UserId::parse(id).ok()?;
    (id.server_name() == owner.server_name() && !id.localpart().starts_with("_hagency_"))
        .then_some(id)
}

fn check_account(client: &Client, owner: &str) -> Result<(), String> {
    if client.user_id().map(|id| id.as_str()) != Some(owner) {
        return Err("management_scope_changed".into());
    }
    Ok(())
}

pub(super) async fn verify(client: &Client, owner: &str, id: &str) -> Result<(), String> {
    check_account(client, owner)?;
    let id = local_person(owner, id).ok_or("invalid_grant_manager")?;
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        client.account().fetch_user_profile_of(&id),
    )
    .await
    .map_err(|_| "grant_people_unavailable")?;
    check_account(client, owner)?;
    result.map(|_| ()).map_err(|error| {
        if matches!(error.client_api_error_kind(), Some(ErrorKind::NotFound)) {
            "grant_person_not_found".into()
        } else {
            "grant_people_unavailable".into()
        }
    })
}

pub(super) async fn search(client: &Client, owner: &str, query: &str) -> Result<People, String> {
    check_account(client, owner)?;
    let query = query.trim();
    if query.len() > 255 {
        return Err("invalid_grant_manager".into());
    }
    let response = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        client.search_users(query, 100),
    )
    .await
    .map_err(|_| "grant_people_unavailable")?
    .map_err(|_| "grant_people_unavailable")?;
    check_account(client, owner)?;
    let mut people = People {
        items: response
            .results
            .into_iter()
            .filter(|user| local_person(owner, user.user_id.as_str()).is_some())
            .map(|user| Person {
                id: user.user_id.to_string(),
                display_name: user.display_name,
            })
            .collect(),
        limited: response.limited,
    };
    // An exact username also works for a registered local person who is not
    // visible in the directory. Never turn a missing profile into a result.
    if !query.is_empty() {
        let server = UserId::parse(owner).map_err(|_| "management_scope_changed")?;
        let name = query.strip_prefix('@').unwrap_or(query);
        let exact = if name.contains(':') {
            format!("@{name}")
        } else {
            format!("@{name}:{}", server.server_name())
        };
        if local_person(owner, &exact).is_some() && !people.items.iter().any(|p| p.id == exact) {
            match verify(client, owner, &exact).await {
                Ok(()) => people.items.push(Person {
                    id: exact,
                    display_name: None,
                }),
                Err(code) if code == "grant_person_not_found" => {}
                Err(code) if people.items.is_empty() => return Err(code),
                Err(_) => {}
            }
        }
    }
    people
        .items
        .sort_by_key(|p| (p.label().to_lowercase(), p.id.clone()));
    let mut seen = std::collections::HashSet::new();
    people.items.retain(|p| seen.insert(p.id.clone()));
    Ok(people)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::search_test_server::DirectoryServer;

    #[test]
    fn candidates_must_belong_to_the_exact_server_and_not_service_identities() {
        let owner = "@owner:river.ominix.io:19443";
        assert!(local_person(owner, "@chris:river.ominix.io:19443").is_some());
        for id in [
            "chris",
            "@chris:river.ominix.io",
            "@chris:other.test",
            "@_hagency_service:river.ominix.io:19443",
        ] {
            assert!(local_person(owner, id).is_none(), "{id}");
        }
    }

    #[tokio::test]
    async fn directory_lists_real_local_people_and_checks_exact_private_accounts() {
        let server = DirectoryServer::new().await;
        let client = server.client().await;
        let owner = "@owner:example.test";
        let page = search(&client, owner, "").await.unwrap();
        assert_eq!(
            page.items,
            vec![Person {
                id: "@chris:example.test".into(),
                display_name: Some("Chris".into())
            }]
        );
        assert!(page.limited);
        assert_eq!(
            search(&client, owner, "Chris").await.unwrap().items.len(),
            1
        );
        assert_eq!(
            search(&client, owner, "private").await.unwrap().items[0].id,
            "@private:example.test"
        );
        assert!(
            search(&client, owner, "not-registered")
                .await
                .unwrap()
                .items
                .is_empty()
        );
        assert_eq!(
            verify(&client, owner, "@not-registered:example.test")
                .await
                .unwrap_err(),
            "grant_person_not_found"
        );
        assert_eq!(
            verify(&client, owner, "@offline:example.test")
                .await
                .unwrap_err(),
            "grant_people_unavailable"
        );
        let before = server.requests.lock().unwrap().len();
        assert_eq!(
            verify(&client, owner, "@chris:remote.test")
                .await
                .unwrap_err(),
            "invalid_grant_manager"
        );
        assert_eq!(
            search(&client, "@another:example.test", "")
                .await
                .unwrap_err(),
            "management_scope_changed"
        );
        assert_eq!(
            server.requests.lock().unwrap().len(),
            before,
            "mismatched identities must not make requests"
        );
    }
}
