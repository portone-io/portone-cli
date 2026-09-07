pub mod fields;
pub mod graphql;

use std::io::Write;
use std::time::Duration;

use clap::Args;
use serde_json::{Map, Value};

use crate::auth;
use crate::cmdutil::AuthOpts;
use crate::error::CliError;
use crate::factory::Factory;
use crate::http::cache::Cache;
use crate::http::pagination::{Advance, Paginator};
use crate::http::response::{self, HttpResponse};
use crate::http::{request, verbose};
use crate::output;
use crate::ui::pager::Pager;

const LONG_ABOUT: &str = r#"Makes an authenticated HTTP request to the PortOne V2 API and prints the response.

The `<ENDPOINT>` argument can be a REST path such as `/payments/{paymentId}`
(replace placeholders with actual values), a full URL, or `graphql` for the
GraphQL API. Paths are appended to `--base-url`, which defaults to
`https://api.portone.io`. Full URLs are used as-is. The Authorization header is
not sent when a full URL has a different origin from the base URL.

The default HTTP method is `GET`, or `POST` when fields or `--input` provide a
request body. Override the method with `--method`. PortOne V2 list endpoints
accept filters in a GET request body, so pass `-X GET` when sending fields to
one of these endpoints.

Pass `-f/--raw-field` values in `key=value` format to add string fields.
`-F/--field` performs type conversion based on the value:

- `true`, `false`, `null`, and integer values are converted to JSON types;
- values beginning with `@` are read from the remaining file path, and `@-`
  reads from standard input.

Use `key[subkey]=value` for nested values, repeated `key[]=value` fields for
arrays, and `key[]` without a value for an empty array.

For GraphQL requests, every field other than `query` and `operationName` is
sent as a GraphQL variable. A response with an `errors` array exits with status
1 even when the HTTP status is 200.

Pass a preconstructed body with `--input <FILE>`, or use `-` to read from
standard input. `--input` cannot be combined with field flags or `--paginate`.

With `--paginate`, requests continue until there are no more pages. REST
pagination uses either `page.totalCount` for offset pagination or
`items[].cursor` for cursor pagination. When no `page` field is supplied,
offset pagination starts with `number=0, size=100`. GraphQL pagination requires
an `$endCursor: String` variable and a `pageInfo { hasNextPage endCursor }`
selection. Each page is printed as a separate JSON value; use `--slurp` to wrap
all pages in one array.

`-q/--jq` uses the embedded jaq engine and cannot be combined with `--slurp`.
Only one of `--jq`, `--silent`, or `--verbose` may be used at a time.

Environment variables:

- `PORTONE_ACCESS_TOKEN`: console access token (takes precedence over profiles; not refreshed)
- `PORTONE_API_BASE`: API base URL (`--base-url` > environment > profile > default)
- `PORTONE_CONFIG_DIR`: configuration directory
- `PORTONE_CACHE_DIR`: response cache directory used by `--cache`
- `PORTONE_PAGER`, `PAGER`: pager for TTY output (`cat` or an empty value disables it)
- `NO_COLOR`, `CLICOLOR_FORCE`: control color output"#;

const EXAMPLES: &str = r#"# Get a payment (replace the placeholder with an actual ID)
$ portone api /payments/{paymentId}

# Add query parameters directly to the path
$ portone api '/payments/{paymentId}?storeId=store-xxx'

# List payments with filters in a GET request body
$ portone api /payments -X GET -F 'page[size]=10' -F 'filter[isTest]=true'

# Pass an array field
$ portone api /payments -X GET \
  -F 'filter[methods][]=CARD' -F 'filter[methods][]=EASY_PAY'

# Cancel a payment (fields switch the request to POST)
$ portone api /payments/{paymentId}/cancel -f reason='Customer request'

# Read a JSON request body from a file or standard input
$ portone api /payments/{paymentId}/cancel --input cancel.json
$ echo '{"reason":"Customer request"}' | portone api /payments/{paymentId}/cancel --input -

# Add a custom header
$ portone api /payments/{paymentId}/cancel -f reason=duplicate-payment \
  -H 'Idempotency-Key: abc123'

# Fetch every page and print payment IDs
$ portone api /payments -X GET --paginate -q '.items[].id'

# Wrap all pages from a cursor-based endpoint in one JSON array
$ portone api /payments-by-cursor -X GET --paginate --slurp

# Cache a response for one hour
$ portone api /payments/{paymentId} --cache 1h

# Make a GraphQL query
$ portone api graphql \
  -f query='query { merchant { ... on Merchant { id plainId } } }'

# Pass GraphQL variables (all fields other than query become variables)
$ portone api graphql -f id='<merchant-global-id>' -f query='
  query($id: ID!) { node(id: $id) { ... on Merchant { plainId } } }
'

# Paginate GraphQL results and build an object variable from nested fields
$ portone api graphql --paginate --slurp \
  -f storeId='<store-global-id>' \
  -F 'filter[statuses][]=IN_PROGRESS' -F 'filter[cardCompanies][]' \
  -f query='
  query($storeId: ID!, $filter: PromotionFilterInput!, $endCursor: String) {
    node(id: $storeId) {
      ... on Store {
        promotions(filter: $filter, first: 50, after: $endCursor) {
          edges { node { id name status } }
          pageInfo { hasNextPage endCursor }
        }
      }
    }
  }'"#;

#[derive(Debug, Args)]
#[command(long_about = LONG_ABOUT, after_long_help = EXAMPLES)]
pub struct ApiArgs {
    #[arg(help = "Endpoint path, full URL, or graphql for the GraphQL API")]
    pub endpoint: String,

    #[arg(
        short = 'X',
        long,
        help = "HTTP method for the request (default: GET, or POST with fields)"
    )]
    pub method: Option<String>,

    #[arg(
        short = 'F',
        long = "field",
        value_name = "key=value",
        help = "Add a typed request field in key=value format (supports @path, @-, integers, true, false, and null)"
    )]
    pub fields: Vec<String>,

    #[arg(
        short = 'f',
        long = "raw-field",
        value_name = "key=value",
        help = "Add a string request field in key=value format"
    )]
    pub raw_fields: Vec<String>,

    #[arg(
        short = 'H',
        long = "header",
        value_name = "key:value",
        help = "Add an HTTP request header in key:value format"
    )]
    pub headers: Vec<String>,

    #[arg(
        long,
        value_name = "FILE",
        help = "File to use as the request body (use \"-\" for standard input)"
    )]
    pub input: Option<String>,

    #[arg(
        short = 'i',
        long,
        help = "Include the response status line and headers in the output"
    )]
    pub include: bool,

    #[arg(long, help = "Make additional requests to fetch all pages of results")]
    pub paginate: bool,

    #[arg(
        long,
        help = "Wrap all paginated JSON values in a single array (requires --paginate)"
    )]
    pub slurp: bool,

    #[arg(
        short = 'q',
        long = "jq",
        value_name = "EXPR",
        help = "Query the response using jq syntax"
    )]
    pub jq: Option<String>,

    #[arg(
        long,
        value_name = "TTL",
        value_parser = parse_duration,
        help = "Cache the response for a duration such as 3600s, 60m, or 1h"
    )]
    pub cache: Option<Duration>,

    #[arg(long, help = "Do not print the response body")]
    pub silent: bool,

    #[arg(
        long,
        help = "Include the full HTTP request and response in the output"
    )]
    pub verbose: bool,

    #[arg(long, help = "Allow printing terminal escape sequences")]
    pub allow_escape_sequences: bool,

    #[command(flatten)]
    pub auth: AuthOpts,
}

impl ApiArgs {
    fn is_graphql(&self) -> bool {
        self.endpoint == "graphql"
    }
}

fn parse_duration(value: &str) -> Result<Duration, String> {
    humantime::parse_duration(value).map_err(|err| err.to_string())
}

pub fn run(f: &mut Factory, args: ApiArgs) -> Result<(), CliError> {
    validate(&args)?;

    let mut params = {
        let mut stdin = std::io::stdin().lock();
        fields::parse_fields(&args.raw_fields, &args.fields, &mut stdin)?
    };

    let input_body = match &args.input {
        Some(path) => Some(request::read_input(path)?),
        None => None,
    };

    let mut config = f.config()?.clone();
    let base_url = auth::resolve_base_url(
        args.auth.base_url.as_deref(),
        args.auth.profile.as_deref(),
        &config,
    );
    let url = request::build_url(&base_url, &args.endpoint);
    let method = request::resolve_method(
        args.method.as_deref(),
        !params.is_empty() || input_body.is_some(),
        args.paginate,
        args.is_graphql(),
    );

    let mut headers = request::parse_headers(&args.headers)?;
    let foreign_origin = args.endpoint.contains("://") && !request::same_origin(&url, &base_url);
    let managed_auth = !request::has_header(&headers, "authorization") && !foreign_origin;
    let agent = f.agent();
    let store = f.secret_store();
    let authorize = |err: &mut dyn Write| -> Result<Option<String>, CliError> {
        if !managed_auth {
            return Ok(None);
        }
        match auth::resolve_fresh(
            &agent,
            store.as_ref(),
            &mut config,
            args.auth.profile.as_deref(),
            err,
        )? {
            Some(resolved) => Ok(Some(resolved.authorization_header())),
            None => Err(CliError::Flag(
                "no credentials found. run `portone auth login` or set PORTONE_ACCESS_TOKEN"
                    .to_string(),
            )),
        }
    };

    let mut paginator = (args.paginate && !args.is_graphql()).then(|| Paginator::new(&mut params));
    let json_body = input_body.is_none() && !params.is_empty();
    request::apply_default_headers(&mut headers, json_body, None);

    let cache = args.cache.map(Cache::new);
    let color = f.io.color_enabled();
    let tty = f.io.stdout_is_tty;

    let io = &mut f.io;
    let mut pager = Pager::start(&mut *io.out, &mut *io.err, tty, !args.silent);
    let result = run_pages(
        &args,
        &agent,
        cache.as_ref(),
        &method,
        &url,
        headers,
        authorize,
        input_body.as_deref(),
        &mut params,
        paginator.as_mut(),
        color,
        tty,
        &mut pager,
        &mut *io.err,
    );

    let finish = pager.finish();
    result?;
    finish?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn run_pages(
    args: &ApiArgs,
    agent: &ureq::Agent,
    cache: Option<&Cache>,
    method: &str,
    url: &str,
    mut headers: Vec<(String, String)>,
    mut authorize: impl FnMut(&mut dyn Write) -> Result<Option<String>, CliError>,
    input_body: Option<&[u8]>,
    params: &mut Map<String, Value>,
    mut paginator: Option<&mut Paginator>,
    color: bool,
    tty: bool,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<(), CliError> {
    let mut pipeline = output::Pipeline::new(args.jq.as_deref(), args.slurp, color, tty)?;
    let discard_body = args.silent || args.verbose;
    let is_graphql = args.is_graphql();
    let mut first_page = true;

    loop {
        if let Some(authorization) = authorize(err)? {
            headers.retain(|(name, _)| !name.eq_ignore_ascii_case("authorization"));
            headers.push(("Authorization".to_string(), authorization));
        }
        let body_bytes: Option<Vec<u8>> = if let Some(input) = input_body {
            Some(input.to_vec())
        } else if params.is_empty() {
            None
        } else if is_graphql {
            Some(
                serde_json::to_vec(&graphql::group_variables(params))
                    .map_err(anyhow::Error::from)?,
            )
        } else {
            Some(serde_json::to_vec(&*params).map_err(anyhow::Error::from)?)
        };

        if args.verbose {
            verbose::log_request(out, method, url, &headers, body_bytes.as_deref())?;
        }

        let resp = fetch(
            agent,
            cache,
            method,
            url,
            &headers,
            body_bytes.as_deref(),
            is_graphql,
        )?;

        if args.verbose {
            verbose::log_response(out, &resp)?;
        }

        if args.include && !args.verbose {
            if !first_page {
                writeln!(out)?;
            }
            response::write_headers(out, &resp, color)?;
        }

        let mut server_error: Option<String> = None;
        if resp.status != 204 {
            let is_json = resp.is_json();
            if is_json && !method.eq_ignore_ascii_case("HEAD") {
                if is_graphql {
                    server_error = graphql::error_message(&resp.body);
                }
                if server_error.is_none() && resp.status >= 400 {
                    server_error = response::parse_error_message(&resp.body)
                        .map(|msg| format!("{msg} (HTTP {})", resp.status));
                }
            }

            if !discard_body {
                if is_json {
                    if resp.status > 299 || server_error.is_some() {
                        output::emit_json_plain(out, &resp.body, color)?;
                    } else {
                        pipeline.emit_json(out, &resp.body)?;
                    }
                } else {
                    output::emit_raw(out, &resp.body, tty, args.allow_escape_sequences)?;
                }
            }

            if server_error.is_none() && resp.status > 299 {
                server_error = Some(format!("HTTP {}", resp.status));
            }
        }
        if let Some(message) = server_error {
            let message = if args.allow_escape_sequences {
                message
            } else {
                output::escape_controls(&message)
            };
            let _ = writeln!(err, "portone: {message}");
            return Err(CliError::Silent);
        }

        if is_graphql && args.paginate {
            if resp.status == 204 {
                break;
            }
            let cursor = if resp.is_json() {
                serde_json::from_slice::<Value>(&resp.body)
                    .ok()
                    .as_ref()
                    .and_then(graphql::find_end_cursor)
            } else {
                None
            };
            match cursor {
                Some(cursor) => {
                    if params.get("endCursor").and_then(Value::as_str) == Some(cursor.as_str()) {
                        let _ =
                            writeln!(err, "portone: pagination cursor did not advance; stopping");
                        break;
                    }
                    params.insert("endCursor".to_string(), Value::String(cursor));
                }
                None => break,
            }
        } else {
            match paginator.as_deref_mut() {
                None => break,
                Some(p) => {
                    if resp.status == 204 {
                        break;
                    }
                    let parsed = if resp.is_json() {
                        serde_json::from_slice::<Value>(&resp.body).ok()
                    } else {
                        None
                    };
                    let Some(value) = parsed else {
                        let _ = writeln!(
                            err,
                            "portone: cannot determine pagination scheme; stopping after first page"
                        );
                        break;
                    };
                    match p.advance(params, &value) {
                        Advance::Next => {}
                        Advance::Done => break,
                        Advance::Stop(message) => {
                            let _ = writeln!(err, "portone: {message}");
                            break;
                        }
                    }
                }
            }
        }
        first_page = false;
    }

    if !discard_body {
        pipeline.finish(out)?;
    }
    Ok(())
}

fn cacheable_method(method: &str, is_graphql: bool) -> bool {
    is_graphql || method.eq_ignore_ascii_case("GET") || method.eq_ignore_ascii_case("HEAD")
}

fn fetch(
    agent: &ureq::Agent,
    cache: Option<&Cache>,
    method: &str,
    url: &str,
    headers: &[(String, String)],
    body: Option<&[u8]>,
    is_graphql: bool,
) -> Result<HttpResponse, CliError> {
    let key = cache.map(|_| {
        Cache::key(
            method,
            url,
            request::header_value(headers, "accept").unwrap_or(""),
            request::header_value(headers, "authorization").unwrap_or(""),
            body.unwrap_or(&[]),
        )
    });
    if let (Some(cache), Some(key)) = (cache, &key)
        && let Some(hit) = cache.lookup(key)
    {
        return Ok(hit.into());
    }

    let resp = request::send(agent, method, url, headers, body)?;

    if let (Some(cache), Some(key)) = (cache, &key) {
        cache.store(key, cacheable_method(method, is_graphql), &resp.to_cached());
    }
    Ok(resp)
}

fn validate(args: &ApiArgs) -> Result<(), CliError> {
    if args.paginate
        && !args.is_graphql()
        && args
            .method
            .as_deref()
            .is_some_and(|m| !m.eq_ignore_ascii_case("GET"))
    {
        return Err(CliError::Flag(
            "the `--paginate` option is not supported for non-GET requests".to_string(),
        ));
    }
    if args.paginate && args.input.is_some() {
        return Err(CliError::Flag(
            "the `--paginate` option is not supported with `--input`".to_string(),
        ));
    }
    if args.slurp && args.jq.is_some() {
        return Err(CliError::Flag(
            "the `--slurp` option is not supported with `--jq`".to_string(),
        ));
    }
    if args.slurp && !args.paginate {
        return Err(CliError::Flag(
            "`--paginate` required when passing `--slurp`".to_string(),
        ));
    }
    if [args.jq.is_some(), args.silent, args.verbose]
        .iter()
        .filter(|&&flag| flag)
        .count()
        > 1
    {
        return Err(CliError::Flag(
            "only one of `--jq`, `--silent`, or `--verbose` may be used".to_string(),
        ));
    }
    if args.input.is_some() && !(args.fields.is_empty() && args.raw_fields.is_empty()) {
        return Err(CliError::Flag(
            "the `--input` option is not supported with `--field` or `--raw-field`".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::ui::IoStreams;

    fn base_args() -> ApiArgs {
        ApiArgs {
            endpoint: "/payments".to_string(),
            method: None,
            fields: Vec::new(),
            raw_fields: Vec::new(),
            headers: Vec::new(),
            input: None,
            include: false,
            paginate: false,
            slurp: false,
            jq: None,
            cache: None,
            silent: false,
            verbose: false,
            allow_escape_sequences: false,
            auth: AuthOpts::default(),
        }
    }

    fn flag_message(result: Result<(), CliError>) -> String {
        match result {
            Err(CliError::Flag(msg)) => msg,
            other => panic!("expected Flag error, got {other:?}"),
        }
    }

    #[test]
    fn validate_accepts_defaults() {
        assert!(validate(&base_args()).is_ok());
    }

    #[test]
    fn validate_rejects_paginate_with_non_get_method() {
        let mut args = base_args();
        args.paginate = true;
        args.method = Some("post".to_string());
        assert_eq!(
            flag_message(validate(&args)),
            "the `--paginate` option is not supported for non-GET requests"
        );

        args.method = Some("get".to_string());
        assert!(validate(&args).is_ok());
    }

    #[test]
    fn validate_allows_paginate_with_post_for_graphql() {
        let mut args = base_args();
        args.endpoint = "graphql".to_string();
        args.paginate = true;
        args.method = Some("post".to_string());
        assert!(validate(&args).is_ok());
    }

    #[test]
    fn validate_rejects_paginate_with_input_for_graphql() {
        let mut args = base_args();
        args.endpoint = "graphql".to_string();
        args.paginate = true;
        args.input = Some("body.json".to_string());
        assert_eq!(
            flag_message(validate(&args)),
            "the `--paginate` option is not supported with `--input`"
        );
    }

    #[test]
    fn cacheable_method_allows_get_head_and_graphql() {
        assert!(cacheable_method("GET", false));
        assert!(cacheable_method("get", false));
        assert!(cacheable_method("Head", false));
        assert!(!cacheable_method("POST", false));
        assert!(cacheable_method("POST", true));
    }

    #[test]
    fn validate_rejects_paginate_with_input() {
        let mut args = base_args();
        args.paginate = true;
        args.input = Some("body.json".to_string());
        assert_eq!(
            flag_message(validate(&args)),
            "the `--paginate` option is not supported with `--input`"
        );
    }

    #[test]
    fn validate_rejects_slurp_with_jq() {
        let mut args = base_args();
        args.paginate = true;
        args.slurp = true;
        args.jq = Some(".".to_string());
        assert_eq!(
            flag_message(validate(&args)),
            "the `--slurp` option is not supported with `--jq`"
        );
    }

    #[test]
    fn validate_requires_paginate_for_slurp() {
        let mut args = base_args();
        args.slurp = true;
        assert_eq!(
            flag_message(validate(&args)),
            "`--paginate` required when passing `--slurp`"
        );
    }

    #[test]
    fn validate_rejects_multiple_output_modes() {
        let mut args = base_args();
        args.silent = true;
        args.verbose = true;
        assert_eq!(
            flag_message(validate(&args)),
            "only one of `--jq`, `--silent`, or `--verbose` may be used"
        );

        let mut args = base_args();
        args.jq = Some(".".to_string());
        args.silent = true;
        assert!(validate(&args).is_err());
    }

    #[test]
    fn validate_rejects_input_with_fields() {
        let mut args = base_args();
        args.input = Some("-".to_string());
        args.fields = vec!["a=1".to_string()];
        assert_eq!(
            flag_message(validate(&args)),
            "the `--input` option is not supported with `--field` or `--raw-field`"
        );
    }

    #[test]
    fn run_streams_response_body_to_io_out() {
        let server = httpmock::MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(httpmock::Method::GET).path("/payments/pay-1");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"id":"pay-1"}"#);
        });

        let (io, bufs) = IoStreams::test();
        let mut f = Factory::with_config(io, Config::default());
        let mut args = base_args();
        args.endpoint = "/payments/pay-1".to_string();
        args.auth.base_url = Some(server.base_url());
        args.headers = vec!["Authorization: Bearer test-token".to_string()];

        run(&mut f, args).unwrap();
        mock.assert();
        assert_eq!(bufs.out(), r#"{"id":"pay-1"}"#);
        assert!(bufs.err().is_empty());
    }

    #[test]
    fn run_escapes_graphql_error_controls_unless_explicitly_allowed() {
        let server = httpmock::MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(httpmock::Method::POST).path("/graphql");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"errors":[{"message":"\u001b]52;c;cHduZWQ=\u0007\u009b2Jhi"}]}"#);
        });

        for allow_escape in [false, true] {
            let (io, bufs) = IoStreams::test();
            let mut f = Factory::with_config(io, Config::default());
            let mut args = base_args();
            args.endpoint = "graphql".to_string();
            args.raw_fields = vec!["query=query{x}".to_string()];
            args.auth.base_url = Some(server.base_url());
            args.headers = vec!["Authorization: Bearer test-token".to_string()];
            args.allow_escape_sequences = allow_escape;

            let result = run(&mut f, args);
            assert!(matches!(result, Err(CliError::Silent)));
            let err = bufs.err();
            if allow_escape {
                assert_eq!(err, "portone: \u{1b}]52;c;cHduZWQ=\u{7}\u{9b}2Jhi\n");
            } else {
                assert_eq!(err, "portone: \\u{1b}]52;c;cHduZWQ=\\u{7}\\u{9b}2Jhi\n");
                assert!(!err.trim_end().chars().any(char::is_control));
            }
        }
        mock.assert_calls(2);
    }

    #[test]
    fn run_escapes_rest_error_controls() {
        let server = httpmock::MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(httpmock::Method::GET).path("/payments/x");
            then.status(404)
                .header("content-type", "application/json")
                .body(r#"{"message":"\u001b]52;c;cHduZWQ=\u0007not found"}"#);
        });

        let (io, bufs) = IoStreams::test();
        let mut f = Factory::with_config(io, Config::default());
        let mut args = base_args();
        args.endpoint = "/payments/x".to_string();
        args.auth.base_url = Some(server.base_url());
        args.headers = vec!["Authorization: Bearer test-token".to_string()];

        let result = run(&mut f, args);
        assert!(matches!(result, Err(CliError::Silent)));
        mock.assert();
        let err = bufs.err();
        assert_eq!(
            err,
            "portone: \\u{1b}]52;c;cHduZWQ=\\u{7}not found (HTTP 404)\n"
        );
        assert!(!err.trim_end().chars().any(char::is_control));
    }

    #[test]
    fn run_reports_http_error_on_io_err() {
        let server = httpmock::MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(httpmock::Method::GET).path("/payments/missing");
            then.status(404)
                .header("content-type", "application/json")
                .body(r#"{"type":"PAYMENT_NOT_FOUND"}"#);
        });

        let (io, bufs) = IoStreams::test();
        let mut f = Factory::with_config(io, Config::default());
        let mut args = base_args();
        args.endpoint = "/payments/missing".to_string();
        args.auth.base_url = Some(server.base_url());
        args.headers = vec!["Authorization: Bearer test-token".to_string()];

        let result = run(&mut f, args);
        assert!(matches!(result, Err(CliError::Silent)));
        mock.assert();
        assert_eq!(bufs.out(), r#"{"type":"PAYMENT_NOT_FOUND"}"#);
        assert!(bufs.err().contains("portone: "));
        assert!(bufs.err().contains("HTTP 404"));
    }
}
