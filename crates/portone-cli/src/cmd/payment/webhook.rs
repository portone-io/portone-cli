use std::io::Write;

use clap::{Args, Subcommand};
use serde_json::{Value, json};

use super::{CommonArgs, TargetArgs, nonempty, object_member, store_query};
use crate::error::CliError;
use crate::factory::Factory;
use crate::http::client::Client;
use crate::output::resource::{self, ResourceKind, ResourceOutput};

#[derive(Debug, Args)]
pub struct WebhookArgs {
    #[command(subcommand)]
    pub command: WebhookCommand,
}

#[derive(Debug, Subcommand)]
pub enum WebhookCommand {
    #[command(about = "List webhooks for a payment", visible_alias = "ls")]
    List(TargetArgs),
    #[command(about = "Resend a payment webhook")]
    Resend(ResendArgs),
}

#[derive(Debug, Args)]
pub struct ResendArgs {
    #[arg(value_name = "PAYMENT_ID", help = "Merchant-assigned payment ID")]
    pub payment_id: String,
    #[arg(
        long,
        value_name = "WEBHOOK_ID",
        help = "Webhook to resend (default: the most recent webhook)"
    )]
    pub webhook_id: Option<String>,
    #[command(flatten)]
    pub output: ResourceOutput,
}

pub fn run(f: &mut Factory, common: &CommonArgs, args: WebhookArgs) -> Result<(), CliError> {
    match args.command {
        WebhookCommand::List(args) => list(f, common, args),
        WebhookCommand::Resend(args) => resend(f, common, args),
    }
}

fn list(f: &mut Factory, common: &CommonArgs, args: TargetArgs) -> Result<(), CliError> {
    nonempty(&args.payment_id, "PAYMENT_ID")?;
    args.output.validate(ResourceKind::Webhook)?;
    let store = common.resolve_store(f)?;
    let mut client = Client::new(f, &common.auth())?;
    let payment = client.request(
        &mut *f.io.err,
        "GET",
        &["payments", &args.payment_id],
        &store_query(store.as_deref()),
        None,
    )?;
    let webhooks = match payment.get("webhooks") {
        None | Some(Value::Null) => Value::Array(Vec::new()),
        Some(Value::Array(items)) => Value::Array(items.clone()),
        Some(_) => {
            return Err(CliError::Other(anyhow::anyhow!(crate::message!(
                "payment-response-field",
                field = "webhooks"
            ))));
        }
    };
    resource::write(f, &args.output, ResourceKind::Webhook, &webhooks)
}

fn resend(f: &mut Factory, common: &CommonArgs, args: ResendArgs) -> Result<(), CliError> {
    nonempty(&args.payment_id, "PAYMENT_ID")?;
    args.output.validate(ResourceKind::Webhook)?;
    let mut body = json!({});
    if let Some(id) = &args.webhook_id {
        nonempty(id, "--webhook-id")?;
        body["webhookId"] = json!(id);
    }
    if let Some(store) = common.resolve_store(f)? {
        body["storeId"] = json!(store);
    }
    let mut client = Client::new(f, &common.auth())?;
    let response = client.request(
        &mut *f.io.err,
        "POST",
        &["payments", &args.payment_id, "resend-webhook"],
        &[],
        Some(&body),
    )?;
    let webhook = object_member(&response, "webhook")?;
    resource::write(f, &args.output, ResourceKind::Webhook, webhook)?;
    if matches!(
        webhook.get("status").and_then(Value::as_str),
        Some("FAILED_NOT_OK_RESPONSE" | "FAILED_UNEXPECTED_ERROR")
    ) {
        writeln!(
            f.io.err,
            "{}",
            crate::tr!(f.localizer, "payment-webhook-failed")
        )?;
        return Err(CliError::Silent);
    }
    Ok(())
}
