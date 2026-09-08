use std::io::Write;
use std::rc::Rc;
use std::sync::Arc;

use serde_json::Value;
use url::Url;

use crate::auth::{self, store::SecretStore};
use crate::cmdutil::AuthOpts;
use crate::config::Config;
use crate::error::CliError;
use crate::factory::Factory;
use crate::http::request;
use crate::i18n::Localizer;
use crate::output::escape_controls;

pub struct Client {
    agent: ureq::Agent,
    config: Config,
    secrets: Rc<dyn SecretStore>,
    profile: Option<String>,
    base_url: Url,
    localizer: Arc<Localizer>,
}

impl Client {
    pub fn new(f: &Factory, opts: &AuthOpts) -> Result<Self, CliError> {
        let config = f.config()?.clone();
        let base =
            auth::resolve_base_url(opts.base_url.as_deref(), opts.profile.as_deref(), &config);
        let base_url = Url::parse(&base).map_err(anyhow::Error::from)?;
        if !matches!(base_url.scheme(), "http" | "https") || base_url.host_str().is_none() {
            return Err(CliError::Other(anyhow::anyhow!(crate::message!(
                "resource-invalid-base-url"
            ))));
        }
        Ok(Self {
            agent: f.agent(),
            config,
            secrets: f.secret_store(),
            profile: opts.profile.clone(),
            base_url,
            localizer: f.localizer.clone(),
        })
    }

    pub fn request(
        &mut self,
        err: &mut dyn Write,
        method: &str,
        path: &[&str],
        query: &[(&str, &str)],
        body: Option<&Value>,
    ) -> Result<Value, CliError> {
        let url = endpoint_url(&self.base_url, path, query)?;
        let authorization = auth::resolve_fresh_localized(
            &self.agent,
            self.secrets.as_ref(),
            &mut self.config,
            self.profile.as_deref(),
            err,
            &self.localizer,
        )?
        .ok_or_else(|| CliError::Flag(crate::tr!(self.localizer, "core-credentials-missing")))?;
        let mut headers = vec![("Accept".to_string(), "application/json".to_string())];
        request::apply_default_headers(
            &mut headers,
            body.is_some(),
            Some(authorization.authorization_header()),
        );
        let bytes = body
            .map(serde_json::to_vec)
            .transpose()
            .map_err(anyhow::Error::from)?;
        let response = request::send(
            &self.agent,
            method,
            url.as_str(),
            &headers,
            bytes.as_deref(),
        )?;
        let parsed = serde_json::from_slice::<Value>(&response.body);
        if !(200..300).contains(&response.status) {
            let details = parsed.as_ref().ok().map(error_details).unwrap_or_default();
            return Err(CliError::Other(anyhow::anyhow!(crate::message!(
                "resource-http-error",
                status = response.status,
                detail = details
            ))));
        }
        parsed.map_err(|error| {
            CliError::Other(anyhow::anyhow!(crate::message!(
                "core-response-parse",
                error = error
            )))
        })
    }
}

fn endpoint_url(base: &Url, path: &[&str], query: &[(&str, &str)]) -> Result<Url, CliError> {
    // URL parsing removes some controls, which could silently change the target ID.
    if path
        .iter()
        .any(|segment| matches!(*segment, "." | "..") || segment.chars().any(char::is_control))
    {
        return Err(CliError::Other(anyhow::anyhow!(crate::message!(
            "resource-invalid-path"
        ))));
    }
    let mut url = base.clone();
    url.set_query(None);
    url.set_fragment(None);
    // Each ID is one segment, even when it contains a slash, query, or fragment.
    url.path_segments_mut()
        .map_err(|()| {
            CliError::Other(anyhow::anyhow!(crate::message!(
                "resource-invalid-base-url"
            )))
        })?
        .pop_if_empty()
        .extend(path.iter().copied());
    if !query.is_empty() {
        url.query_pairs_mut().extend_pairs(query.iter().copied());
    }
    Ok(url)
}

fn error_details(value: &Value) -> String {
    ["type", "message", "pgCode", "pgMessage"]
        .iter()
        .filter_map(|key| value.get(*key).and_then(Value::as_str))
        .filter(|text| !text.is_empty())
        .map(escape_controls)
        .collect::<Vec<_>>()
        .join(": ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn endpoint_encodes_ids_without_changing_base_path_or_query_scope() {
        let base = Url::parse("https://example.test/v2/?old=1#old").unwrap();
        let url = endpoint_url(
            &base,
            &["payments", "order/a?b#c%20"],
            &[("storeId", "a&b")],
        )
        .unwrap();
        assert_eq!(
            url.as_str(),
            "https://example.test/v2/payments/order%2Fa%3Fb%23c%2520?storeId=a%26b"
        );
    }

    #[test]
    fn api_errors_retain_type_and_pg_diagnostics_without_terminal_escapes() {
        assert_eq!(
            error_details(
                &json!({"type":"PG_PROVIDER","message":"failed","pgCode":"42","pgMessage":"declined\u{1b}"})
            ),
            "PG_PROVIDER: failed: 42: declined\\u{1b}"
        );
    }

    #[test]
    fn dot_segment_ids_are_rejected_instead_of_requesting_another_resource() {
        let base = Url::parse("https://example.test/").unwrap();
        for id in [".", ".."] {
            assert!(endpoint_url(&base, &["payments", id], &[]).is_err());
        }
    }

    #[test]
    fn control_characters_are_rejected_instead_of_changing_the_target_id() {
        let base = Url::parse("https://example.test/").unwrap();
        for id in [
            "order\n123",
            "order\r123",
            "order\t123",
            ".\t.",
            "order\0",
            "order\u{7f}",
        ] {
            assert!(endpoint_url(&base, &["payments", id, "cancel"], &[]).is_err());
        }
    }
}
