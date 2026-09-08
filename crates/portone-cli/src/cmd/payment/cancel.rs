use std::io::Write;

use clap::Args;
use serde_json::{Map, Value, json};

use super::{CommonArgs, nonempty, object_member, store_query};
use crate::auth;
use crate::error::CliError;
use crate::factory::Factory;
use crate::http::client::Client;
use crate::http::request;
use crate::output::resource::{self, ResourceKind, ResourceOutput};

#[derive(Debug, Args)]
pub struct CancelArgs {
    #[arg(value_name = "PAYMENT_ID", help = "Merchant-assigned payment ID")]
    pub payment_id: String,
    #[arg(
        long,
        required_unless_present = "input",
        conflicts_with = "input",
        help = "Reason for cancelling the payment"
    )]
    pub reason: Option<String>,
    #[arg(long, value_name = "INTEGER", value_parser = clap::value_parser!(i64).range(1..), conflicts_with = "input", help = "Amount to cancel in currency minor units (default: all remaining)")]
    pub amount: Option<i64>,
    #[arg(long, value_name = "INTEGER", value_parser = clap::value_parser!(i64).range(0..), conflicts_with = "input", help = "Tax-free cancellation amount in currency minor units")]
    pub tax_free_amount: Option<i64>,
    #[arg(long, value_name = "INTEGER", value_parser = clap::value_parser!(i64).range(0..), conflicts_with = "input", help = "VAT cancellation amount in currency minor units")]
    pub vat_amount: Option<i64>,
    #[arg(long, value_name = "INTEGER", value_parser = clap::value_parser!(i64).range(0..), conflicts_with = "input", help = "Expected cancellable balance in currency minor units")]
    pub current_cancellable_amount: Option<i64>,
    #[arg(
        long,
        value_name = "FILE",
        help = "Read the cancellation JSON body from a file (use - for stdin)"
    )]
    pub input: Option<String>,
    #[arg(
        long,
        short = 'y',
        help = "Skip confirmation (required when not running interactively)"
    )]
    pub yes: bool,
    #[command(flatten)]
    pub output: ResourceOutput,
}

pub fn run(f: &mut Factory, common: &CommonArgs, args: CancelArgs) -> Result<(), CliError> {
    nonempty(&args.payment_id, "PAYMENT_ID")?;
    args.output.validate(ResourceKind::Cancellation)?;
    if !args.yes && !f.io.can_prompt() {
        return Err(CliError::Other(anyhow::anyhow!(crate::message!(
            "payment-cancel-needs-yes"
        ))));
    }
    let mut body = cancellation_body(&args)?;
    let store = cancellation_store(common.store.as_deref(), body.get("storeId"), || {
        common.resolve_store(f)
    })?;
    if let Some(store) = &store {
        body.insert("storeId".into(), json!(store));
    }
    let mut client = Client::new(f, &common.auth())?;
    if !args.yes {
        let payment = client.request(
            &mut *f.io.err,
            "GET",
            &["payments", &args.payment_id],
            &store_query(store.as_deref()),
            None,
        )?;
        if !confirm(f, common, &args.payment_id, &body, &payment)? {
            writeln!(
                f.io.err,
                "{}",
                crate::tr!(f.localizer, "payment-cancel-aborted")
            )?;
            return Err(CliError::Silent);
        }
    }
    let response = client.request(
        &mut *f.io.err,
        "POST",
        &["payments", &args.payment_id, "cancel"],
        &[],
        Some(&Value::Object(body)),
    )?;
    let cancellation = object_member(&response, "cancellation")?;
    resource::write(f, &args.output, ResourceKind::Cancellation, cancellation)?;
    if cancellation.get("status").and_then(Value::as_str) == Some("FAILED") {
        writeln!(
            f.io.err,
            "{}",
            crate::tr!(f.localizer, "payment-cancel-failed")
        )?;
        return Err(CliError::Silent);
    }
    Ok(())
}

fn cancellation_body(args: &CancelArgs) -> Result<Map<String, Value>, CliError> {
    let body = if let Some(input) = &args.input {
        let bytes = request::read_input(input)?;
        serde_json::from_slice::<Value>(&bytes).map_err(|error| {
            CliError::Other(anyhow::anyhow!(crate::message!(
                "payment-cancel-input-json",
                error = error
            )))
        })?
    } else {
        let mut body = Map::new();
        if let Some(reason) = &args.reason {
            body.insert("reason".into(), json!(reason));
        }
        for (key, value) in [
            ("amount", args.amount),
            ("taxFreeAmount", args.tax_free_amount),
            ("vatAmount", args.vat_amount),
            ("currentCancellableAmount", args.current_cancellable_amount),
        ] {
            if let Some(value) = value {
                body.insert(key.into(), json!(value));
            }
        }
        Value::Object(body)
    };
    validate_body(body)
}

fn validate_body(body: Value) -> Result<Map<String, Value>, CliError> {
    let Value::Object(body) = body else {
        return Err(invalid_field("body"));
    };
    required_string(&body, "reason")?;
    for key in [
        "amount",
        "taxFreeAmount",
        "vatAmount",
        "currentCancellableAmount",
    ] {
        if let Some(value) = body.get(key)
            && !value.as_i64().is_some_and(|value| {
                if key == "amount" {
                    value > 0
                } else {
                    value >= 0
                }
            })
        {
            return Err(CliError::Other(anyhow::anyhow!(crate::message!(
                "payment-cancel-input-amount",
                field = key
            ))));
        }
    }
    for key in ["storeId", "refundEmail"] {
        if body.contains_key(key) {
            required_string(&body, key)?;
        }
    }
    for (key, options) in [
        ("requester", &["CUSTOMER", "ADMIN"][..]),
        ("promotionDiscountRetainOption", &["RETAIN", "RELEASE"][..]),
    ] {
        if let Some(value) = body.get(key)
            && !value.as_str().is_some_and(|value| options.contains(&value))
        {
            return Err(invalid_field(key));
        }
    }
    if let Some(skip) = body.get("skipWebhook")
        && !skip.is_boolean()
    {
        return Err(invalid_field("skipWebhook"));
    }
    if let Some(account) = body.get("refundAccount") {
        let account = account
            .as_object()
            .ok_or_else(|| invalid_field("refundAccount"))?;
        for key in ["bank", "number", "holderName"] {
            required_string(account, key)?;
        }
        if account.contains_key("holderPhoneNumber") {
            required_string(account, "holderPhoneNumber")?;
        }
    }
    Ok(body)
}

fn required_string(body: &Map<String, Value>, field: &str) -> Result<(), CliError> {
    if !body
        .get(field)
        .and_then(Value::as_str)
        .is_some_and(|value| !value.trim().is_empty())
    {
        return Err(invalid_field(field));
    }
    Ok(())
}

fn invalid_field(field: &str) -> CliError {
    CliError::Other(anyhow::anyhow!(crate::message!(
        "payment-cancel-input-field",
        field = field
    )))
}

fn cancellation_store(
    explicit: Option<&str>,
    input: Option<&Value>,
    default: impl FnOnce() -> Result<Option<String>, CliError>,
) -> Result<Option<String>, CliError> {
    let input = input.and_then(Value::as_str);
    if let Some(explicit) = explicit {
        nonempty(explicit, "--store")?;
        if input.is_some_and(|store| store != explicit) {
            return Err(CliError::Other(anyhow::anyhow!(crate::message!(
                "payment-cancel-store-conflict"
            ))));
        }
        return Ok(Some(explicit.to_string()));
    }
    if let Some(input) = input {
        return Ok(Some(input.to_string()));
    }
    default()
}

fn confirm(
    f: &mut Factory,
    common: &CommonArgs,
    payment_id: &str,
    body: &Map<String, Value>,
    payment: &Value,
) -> Result<bool, CliError> {
    let localizer = f.localizer.clone();
    let profile = auth::profile_name(common.profile.as_deref(), f.config()?);
    let store = body
        .get("storeId")
        .or_else(|| payment.get("storeId"))
        .and_then(Value::as_str)
        .unwrap_or("-");
    let environment = payment
        .pointer("/channel/type")
        .and_then(Value::as_str)
        .unwrap_or("-");
    let amount = body
        .get("amount")
        .map(Value::to_string)
        .unwrap_or_else(|| crate::tr!(localizer, "payment-cancel-all-remaining"));
    let currency = payment
        .get("currency")
        .and_then(Value::as_str)
        .unwrap_or("-");
    let reason = body
        .get("reason")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let preview = crate::tr!(
        localizer,
        "payment-cancel-preview",
        profile = resource::cell(&profile),
        store = resource::cell(store),
        payment_id = resource::cell(payment_id),
        environment = resource::cell(environment),
        amount = amount,
        currency = resource::cell(currency),
        reason = resource::cell(reason)
    );
    writeln!(f.io.err, "{}", crate::output::escape_controls(&preview))?;
    write!(
        f.io.err,
        "{} ",
        crate::tr!(localizer, "payment-cancel-confirm")
    )?;
    f.io.err.flush()?;
    let mut answer = String::new();
    std::io::stdin().read_line(&mut answer)?;
    Ok(affirmative(&answer))
}

fn affirmative(answer: &str) -> bool {
    matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmd::payment::{PaymentArgs, PaymentCommand};
    use clap::Parser;

    #[derive(Parser)]
    struct TestCli {
        #[command(flatten)]
        payment: PaymentArgs,
    }

    #[test]
    fn validates_required_reason_and_exact_integer_amounts() {
        for body in [
            json!([]),
            json!({}),
            json!({"reason":" "}),
            json!({"reason":"test","amount":0}),
            json!({"reason":"test","amount":1.5}),
            json!({"reason":"test","amount":"100"}),
            json!({"reason":"test","taxFreeAmount":-1}),
            json!({"reason":"test","amount":9223372036854775808u64}),
        ] {
            assert!(validate_body(body).is_err());
        }
        let body = json!({"reason":"test","amount":9007199254740993i64,"taxFreeAmount":0,"vatAmount":0,"currentCancellableAmount":9007199254740993i64});
        assert_eq!(
            validate_body(body.clone()).unwrap(),
            body.as_object().unwrap().clone()
        );
    }

    #[test]
    fn input_preserves_advanced_fields_and_rejects_invalid_types() {
        let body = json!({"reason":"test","requester":"ADMIN","skipWebhook":false,"refundAccount":{"bank":"SHINHAN","number":"123","holderName":"test"},"promotionDiscountRetainOption":"RETAIN"});
        assert_eq!(
            validate_body(body.clone()).unwrap(),
            body.as_object().unwrap().clone()
        );
        for body in [
            json!({"reason":"test","skipWebhook":"false"}),
            json!({"reason":"test","refundAccount":{"bank":"SHINHAN"}}),
            json!({"reason":"test","requester":"admin"}),
        ] {
            assert!(validate_body(body).is_err());
        }
    }

    #[test]
    fn input_store_takes_precedence_and_explicit_conflicts_fail() {
        assert_eq!(
            cancellation_store(None, Some(&json!("input")), || panic!(
                "default must not be read"
            ))
            .unwrap(),
            Some("input".into())
        );
        assert_eq!(
            cancellation_store(Some("input"), Some(&json!("input")), || panic!(
                "default must not be read"
            ))
            .unwrap(),
            Some("input".into())
        );
        assert!(cancellation_store(Some("flag"), Some(&json!("input")), || Ok(None)).is_err());
        assert_eq!(
            cancellation_store(None, None, || Ok(Some("default".into()))).unwrap(),
            Some("default".into())
        );
    }

    #[test]
    fn input_and_body_flags_are_mutually_exclusive() {
        for flag in [
            "--reason",
            "--amount",
            "--tax-free-amount",
            "--vat-amount",
            "--current-cancellable-amount",
        ] {
            assert!(
                TestCli::try_parse_from([
                    "payment",
                    "cancel",
                    "id",
                    "--input",
                    "body.json",
                    flag,
                    "1"
                ])
                .is_err()
            );
        }
        assert!(TestCli::try_parse_from(["payment", "cancel", "id"]).is_err());
        assert!(TestCli::try_parse_from(["payment", "cancel", "id", "--input", "-"]).is_ok());
        let cli = TestCli::try_parse_from([
            "payment",
            "cancel",
            "id",
            "--reason",
            "test",
            "--amount",
            "9007199254740993",
        ])
        .unwrap();
        let PaymentCommand::Cancel(args) = cli.payment.command else {
            panic!("not cancel")
        };
        assert_eq!(
            cancellation_body(&args).unwrap()["amount"],
            json!(9007199254740993i64)
        );
    }

    #[test]
    fn cancellation_requires_an_explicit_affirmative_answer() {
        for answer in ["", "\n", "n", "no", "unexpected"] {
            assert!(!affirmative(answer));
        }
        for answer in ["y", "Y\n", "yes", " YES "] {
            assert!(affirmative(answer));
        }
    }
}
