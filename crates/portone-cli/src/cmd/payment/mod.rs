mod cancel;
mod list;
mod schema;
mod webhook;

use clap::{Args, Subcommand};
use serde_json::Value;

use crate::auth;
use crate::cmdutil::AuthOpts;
use crate::error::CliError;
use crate::factory::Factory;
use crate::http::client::Client;
use crate::output::resource::{self, ResourceKind, ResourceOutput};

#[derive(Debug, Args)]
pub struct PaymentArgs {
    #[command(flatten)]
    pub common: CommonArgs,
    #[command(subcommand)]
    pub command: PaymentCommand,
}

#[derive(Debug, Clone, Default, Args)]
pub struct CommonArgs {
    #[arg(
        long,
        global = true,
        value_name = "NAME",
        help = "Configuration profile to use"
    )]
    pub profile: Option<String>,
    #[arg(
        long,
        global = true,
        value_name = "URL",
        help = "Base URL for API requests (default: https://api.portone.io)"
    )]
    pub base_url: Option<String>,
    #[arg(
        long,
        global = true,
        value_name = "STORE_ID",
        help = "Store ID (default: PORTONE_STORE_ID or profile store_id)"
    )]
    pub store: Option<String>,
}

impl CommonArgs {
    fn auth(&self) -> AuthOpts {
        AuthOpts {
            profile: self.profile.clone(),
            base_url: self.base_url.clone(),
        }
    }

    fn resolve_store(&self, f: &Factory) -> Result<Option<String>, CliError> {
        if let Some(store) = &self.store {
            nonempty(store, "--store")?;
            return Ok(Some(store.clone()));
        }
        let config = f.config()?;
        let name = auth::profile_name(self.profile.as_deref(), config);
        Ok(default_store(
            std::env::var("PORTONE_STORE_ID").ok().as_deref(),
            config
                .profiles
                .get(&name)
                .and_then(|p| p.store_id.as_deref()),
        ))
    }
}

fn default_store(environment: Option<&str>, profile: Option<&str>) -> Option<String> {
    auth::normalize(environment).or_else(|| auth::normalize(profile))
}

#[derive(Debug, Subcommand)]
pub enum PaymentCommand {
    #[command(about = "List recent payments", visible_alias = "ls")]
    List(list::ListArgs),
    #[command(about = "View a payment")]
    View(TargetArgs),
    #[command(about = "List payment attempts (unstable API)")]
    Transactions(TargetArgs),
    #[command(about = "Cancel a payment")]
    Cancel(cancel::CancelArgs),
    #[command(about = "Inspect and resend payment webhooks")]
    Webhook(webhook::WebhookArgs),
}

#[derive(Debug, Args)]
pub struct TargetArgs {
    #[arg(value_name = "PAYMENT_ID", help = "Merchant-assigned payment ID")]
    pub payment_id: String,
    #[command(flatten)]
    pub output: ResourceOutput,
}

pub fn run(f: &mut Factory, args: PaymentArgs) -> Result<(), CliError> {
    match args.command {
        PaymentCommand::List(target) => list::run(f, &args.common, target),
        PaymentCommand::View(target) => view(f, &args.common, target, false),
        PaymentCommand::Transactions(target) => view(f, &args.common, target, true),
        PaymentCommand::Cancel(target) => cancel::run(f, &args.common, target),
        PaymentCommand::Webhook(target) => webhook::run(f, &args.common, target),
    }
}

fn view(
    f: &mut Factory,
    common: &CommonArgs,
    args: TargetArgs,
    transactions: bool,
) -> Result<(), CliError> {
    nonempty(&args.payment_id, "PAYMENT_ID")?;
    let kind = if transactions {
        ResourceKind::Transaction
    } else {
        ResourceKind::Payment
    };
    args.output.validate(kind)?;
    let store = common.resolve_store(f)?;
    let mut client = Client::new(f, &common.auth())?;
    let mut path = vec!["payments", args.payment_id.as_str()];
    if transactions {
        path.push("transactions");
    }
    let result = client.request(
        &mut *f.io.err,
        "GET",
        &path,
        &store_query(store.as_deref()),
        None,
    )?;
    let data = if transactions {
        array_member(&result, "items")?
    } else {
        &result
    };
    resource::write(f, &args.output, kind, data)
}

fn store_query(store: Option<&str>) -> Vec<(&str, &str)> {
    store.map(|id| vec![("storeId", id)]).unwrap_or_default()
}

fn nonempty(value: &str, field: &str) -> Result<(), CliError> {
    if value.trim().is_empty() {
        return Err(CliError::Other(anyhow::anyhow!(crate::message!(
            "payment-empty-value",
            field = field
        ))));
    }
    Ok(())
}

fn array_member<'a>(data: &'a Value, key: &str) -> Result<&'a Value, CliError> {
    data.get(key).filter(|v| v.is_array()).ok_or_else(|| {
        CliError::Other(anyhow::anyhow!(crate::message!(
            "payment-response-field",
            field = key
        )))
    })
}

fn object_member<'a>(data: &'a Value, key: &str) -> Result<&'a Value, CliError> {
    data.get(key).filter(|v| v.is_object()).ok_or_else(|| {
        CliError::Other(anyhow::anyhow!(crate::message!(
            "payment-response-field",
            field = key
        )))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct TestCli {
        #[command(flatten)]
        payment: PaymentArgs,
    }

    #[test]
    fn store_defaults_ignore_empty_values() {
        assert_eq!(
            default_store(Some("env"), Some("profile")),
            Some("env".into())
        );
        assert_eq!(
            default_store(Some(" "), Some("profile")),
            Some("profile".into())
        );
        assert_eq!(default_store(None, Some(" ")), None);
    }

    #[test]
    fn store_and_profile_flags_work_before_or_after_nested_commands() {
        for args in [
            vec![
                "payment",
                "--store",
                "store",
                "--profile",
                "profile",
                "webhook",
                "list",
                "id",
            ],
            vec![
                "payment",
                "webhook",
                "list",
                "id",
                "--store",
                "store",
                "--profile",
                "profile",
            ],
        ] {
            let cli = TestCli::try_parse_from(args).unwrap();
            assert_eq!(cli.payment.common.store.as_deref(), Some("store"));
            assert_eq!(cli.payment.common.profile.as_deref(), Some("profile"));
        }
    }
}
