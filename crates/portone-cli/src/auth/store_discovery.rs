use std::collections::HashSet;
use std::time::Duration;

use anyhow::{anyhow, bail};
use serde::Deserialize;
use serde_json::Value;

use crate::error::CliError;
use crate::i18n::{LocalizedContext, LocalizedErrorContext, Localizer};
use crate::output::resource::cell;

const STORES_QUERY: &str = "query CliLoginStores { merchant { __typename ... on Merchant { stores { __typename ... on StoresPayload { items { plainId name isRepresentative } } ... on Error { message } } } ... on Error { message } } }";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreSummary {
    pub plain_id: String,
    pub name: String,
    pub is_representative: bool,
}

impl StoreSummary {
    pub fn label(&self, localizer: &Localizer) -> String {
        let suffix = if self.is_representative {
            format!(
                " [{}]",
                crate::tr!(localizer, "store-selection-representative")
            )
        } else {
            String::new()
        };
        format!("{} ({}){suffix}", cell(&self.name), cell(&self.plain_id))
    }
}

pub fn valid_store_id(id: &str) -> bool {
    !id.trim().is_empty() && !id.chars().any(char::is_control)
}

pub fn discover(
    agent: &ureq::Agent,
    base_url: &str,
    access_token: &str,
) -> anyhow::Result<Vec<StoreSummary>> {
    let mut response = agent
        .post(format!("{}/graphql", base_url.trim_end_matches('/')))
        .header("Authorization", &format!("Bearer {access_token}"))
        .config()
        .timeout_global(Some(Duration::from_secs(10)))
        .build()
        .send_json(serde_json::json!({ "query": STORES_QUERY }))
        .lcontext(crate::message!("store-discovery-request-failed"))?;
    let status = response.status().as_u16();
    if !(200..300).contains(&status) {
        bail!(crate::message!("store-discovery-http", status = status));
    }
    let response: Value = response
        .body_mut()
        .read_json()
        .lcontext(crate::message!("store-discovery-parse-failed"))?;
    parse_response(response)
}

fn parse_response(response: Value) -> anyhow::Result<Vec<StoreSummary>> {
    if let Some(errors) = response.get("errors") {
        let errors = errors
            .as_array()
            .ok_or_else(|| anyhow!(crate::message!("store-discovery-malformed")))?;
        if !errors.is_empty() {
            let detail = errors
                .iter()
                .filter_map(|error| error.get("message").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("; ");
            bail!(crate::message!("store-discovery-graphql", detail = detail));
        }
    }
    let merchant = response
        .pointer("/data/merchant")
        .ok_or_else(|| anyhow!(crate::message!("store-discovery-malformed")))?;
    expect_variant(merchant, "Merchant")?;
    let stores = merchant
        .get("stores")
        .ok_or_else(|| anyhow!(crate::message!("store-discovery-malformed")))?;
    expect_variant(stores, "StoresPayload")?;
    let items = stores
        .get("items")
        .ok_or_else(|| anyhow!(crate::message!("store-discovery-malformed")))?;
    let mut items: Vec<StoreSummary> = serde_json::from_value(items.clone())
        .lcontext(crate::message!("store-discovery-malformed"))?;
    let mut ids = HashSet::new();
    if items
        .iter()
        .any(|item| !valid_store_id(&item.plain_id) || !ids.insert(&item.plain_id))
    {
        bail!(crate::message!("store-discovery-malformed"));
    }
    items.sort_by(|a, b| {
        b.is_representative
            .cmp(&a.is_representative)
            .then(a.name.cmp(&b.name))
            .then(a.plain_id.cmp(&b.plain_id))
    });
    Ok(items)
}

fn expect_variant(value: &Value, expected: &str) -> anyhow::Result<()> {
    let kind = value
        .get("__typename")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!(crate::message!("store-discovery-malformed")))?;
    if kind != expected {
        bail!(crate::message!(
            "store-discovery-union",
            kind = kind,
            detail = value.get("message").and_then(Value::as_str).unwrap_or("")
        ));
    }
    Ok(())
}

pub fn preferred_store<'a>(
    stores: &'a [StoreSummary],
    previous: Option<&str>,
) -> Option<&'a StoreSummary> {
    if let Some(store) = previous.and_then(|id| stores.iter().find(|store| store.plain_id == id)) {
        return Some(store);
    }
    let mut representatives = stores.iter().filter(|store| store.is_representative);
    if let Some(store) = representatives.next()
        && representatives.next().is_none()
    {
        return Some(store);
    }
    if stores.len() == 1 {
        return stores.first();
    }
    None
}

pub fn pick_store(
    stores: &[StoreSummary],
    previous: Option<&str>,
    allow_skip: bool,
    localizer: &Localizer,
) -> Result<Option<StoreSummary>, CliError> {
    if stores.is_empty() {
        return Err(CliError::Other(anyhow!(crate::message!(
            "store-selection-empty"
        ))));
    }
    let mut labels = stores
        .iter()
        .map(|store| store.label(localizer))
        .collect::<Vec<_>>();
    if allow_skip {
        labels.push(crate::tr!(localizer, "store-selection-skip"));
    }
    let question = crate::tr!(localizer, "store-selection-question");
    let hint = crate::tr!(localizer, "store-selection-hint");
    let canceled = crate::tr!(localizer, "store-selection-canceled");
    let cursor = previous
        .and_then(|id| stores.iter().position(|store| store.plain_id == id))
        .unwrap_or(0);
    let mut prompt = inquire::Select::new(&question, labels)
        .with_starting_cursor(cursor)
        .with_help_message(&hint);
    prompt.render_config.canceled_prompt_indicator.content = &canceled;
    match prompt.raw_prompt() {
        Ok(selection) => Ok(stores.get(selection.index).cloned()),
        Err(inquire::InquireError::OperationCanceled) => Ok(None),
        Err(error) => Err(CliError::Other(
            anyhow!(error).lcontext(crate::message!("store-selection-failed")),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn store(id: &str, representative: bool) -> StoreSummary {
        StoreSummary {
            plain_id: id.to_string(),
            name: id.to_string(),
            is_representative: representative,
        }
    }

    #[test]
    fn labels_mark_representative_stores_and_escape_terminal_controls() {
        let item = StoreSummary {
            plain_id: "store-plain".to_string(),
            name: "Main\nStore\t\u{1b}[31m".to_string(),
            is_representative: true,
        };
        assert_eq!(
            item.label(&Localizer::english()),
            "Main\\nStore\\t\\u{1b}[31m (store-plain) [representative]"
        );
        assert!(item.label(&Localizer::korean()).contains("[대표]"));
        assert_eq!(
            store("store-child", false).label(&Localizer::english()),
            "store-child (store-child)"
        );
    }

    #[test]
    fn store_ids_must_not_include_terminal_controls() {
        for id in [
            "",
            " ",
            "store\n",
            "store\t",
            "store\r",
            "store\u{1b}",
            "store\u{7f}",
        ] {
            assert!(!valid_store_id(id));
        }
        assert!(valid_store_id("store-plain"));
    }

    #[test]
    fn chooses_previous_then_unique_representative_then_sole_store() {
        let stores = [store("store-main", true), store("store-child", false)];
        assert_eq!(
            preferred_store(&stores, Some("store-child")),
            Some(&stores[1])
        );
        assert_eq!(
            preferred_store(&stores, Some("store-gone")),
            Some(&stores[0])
        );
        assert_eq!(preferred_store(&stores, None), Some(&stores[0]));
        let sole = [store("store-only", false)];
        assert_eq!(preferred_store(&sole, None), Some(&sole[0]));
        assert!(preferred_store(&[], None).is_none());
        assert!(preferred_store(&[store("a", true), store("b", true)], None).is_none());
        assert!(preferred_store(&[store("a", false), store("b", false)], None).is_none());
    }

    #[test]
    fn discovery_uses_bearer_and_plain_ids_and_sorts_representative_first() {
        let server = httpmock::MockServer::start();
        let request = server.mock(|when, then| {
            when.method(httpmock::Method::POST)
                .path("/graphql")
                .header("authorization", "Bearer newly-exchanged-token")
                .json_body(json!({ "query": STORES_QUERY }));
            then.status(200).json_body(json!({"data":{"merchant":{
                "__typename":"Merchant","stores":{"__typename":"StoresPayload","items":[
                    {"id":"global-child","plainId":"store-a","name":"A","isRepresentative":false},
                    {"id":"global-main","plainId":"store-z","name":"Z","isRepresentative":true}
                ]}
            }}}));
        });
        let stores = discover(
            &crate::http::build_agent(),
            &server.base_url(),
            "newly-exchanged-token",
        )
        .unwrap();
        assert_eq!(stores[0].plain_id, "store-z");
        assert_eq!(stores[1].plain_id, "store-a");
        request.assert();
    }

    #[test]
    fn rejects_graphql_and_union_errors_instead_of_treating_them_as_empty_lists() {
        for response in [
            json!({"errors":[{"message":"missing scope"}]}),
            json!({"data":{"merchant":{"__typename":"UnauthorizedError","message":"denied"}}}),
            json!({"data":{"merchant":{"__typename":"Merchant","stores":{"__typename":"ForbiddenError","message":"STORE_READ"}}}}),
            json!({"data":{"merchant":{"__typename":"Merchant","stores":{"__typename":"StoresPayload","items":null}}}}),
            json!({"data":null}),
        ] {
            assert!(parse_response(response).is_err());
        }
        assert!(parse_response(json!({"data":{"merchant":{"__typename":"Merchant","stores":{"__typename":"StoresPayload","items":[]}}}})).unwrap().is_empty());
    }
}
