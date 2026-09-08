use std::io::Write;

use clap::Args;
use serde_json::{Map, Value};
use unicode_width::UnicodeWidthStr;

use crate::error::CliError;
use crate::factory::Factory;
use crate::i18n::Localizer;
use crate::output::{Pipeline, escape_controls};
use crate::ui::pager::Pager;

#[derive(Debug, Clone, Default, Args)]
pub struct ResourceOutput {
    #[arg(long, num_args = 0..=1, default_missing_value = "", value_name = "FIELDS", help = "Output JSON, optionally selecting comma-separated fields")]
    pub json: Option<String>,
    #[arg(
        short = 'q',
        long,
        value_name = "EXPR",
        requires = "json",
        help = "Filter JSON output using a jq expression (requires --json)"
    )]
    pub jq: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum ResourceKind {
    Payment,
    Transaction,
    Cancellation,
    Webhook,
}

impl ResourceKind {
    fn fields(self) -> &'static [&'static str] {
        match self {
            Self::Payment => PAYMENT_FIELDS,
            Self::Transaction => TRANSACTION_FIELDS,
            Self::Cancellation => CANCELLATION_FIELDS,
            Self::Webhook => WEBHOOK_FIELDS,
        }
    }
}

const PAYMENT_FIELDS: &[&str] = &[
    "amount",
    "billingKey",
    "cancellations",
    "cancelledAt",
    "cashReceipt",
    "cashReceiptIssuanceStatus",
    "channel",
    "channelGroup",
    "country",
    "currency",
    "customData",
    "customer",
    "disputes",
    "escrow",
    "failedAt",
    "failure",
    "id",
    "isCulturalExpense",
    "merchantId",
    "method",
    "orderName",
    "origin",
    "paidAt",
    "pgResponse",
    "pgTxId",
    "productCount",
    "products",
    "promotionId",
    "receiptUrl",
    "requestedAt",
    "scheduleId",
    "status",
    "statusChangedAt",
    "storeId",
    "transactionId",
    "updatedAt",
    "version",
    "webhooks",
];
const TRANSACTION_FIELDS: &[&str] = &[
    "amount",
    "billingKey",
    "cancellations",
    "cancelledAt",
    "cashReceipt",
    "cashReceiptIssuanceStatus",
    "channel",
    "channelGroup",
    "country",
    "currency",
    "customData",
    "customer",
    "escrow",
    "failedAt",
    "failure",
    "id",
    "isCulturalExpense",
    "merchantId",
    "method",
    "orderName",
    "paidAt",
    "paymentId",
    "pgResponse",
    "pgTxId",
    "productCount",
    "products",
    "promotionId",
    "receiptUrl",
    "requestedAt",
    "scheduleId",
    "status",
    "statusChangedAt",
    "storeId",
    "updatedAt",
    "version",
    "webhooks",
];
const CANCELLATION_FIELDS: &[&str] = &[
    "cancelledAt",
    "easyPayDiscountAmount",
    "id",
    "pgCancellationId",
    "reason",
    "receiptUrl",
    "requestedAt",
    "status",
    "taxFreeAmount",
    "totalAmount",
    "trigger",
    "vatAmount",
];
const WEBHOOK_FIELDS: &[&str] = &[
    "currentExecutionCount",
    "id",
    "isAsync",
    "maxExecutionCount",
    "paymentStatus",
    "request",
    "response",
    "status",
    "trigger",
    "triggeredAt",
    "url",
];

impl ResourceOutput {
    pub fn validate(&self, kind: ResourceKind) -> Result<(), CliError> {
        if self.jq.is_some() && self.json.is_none() {
            return Err(CliError::Other(anyhow::anyhow!(crate::message!(
                "resource-jq-requires-json"
            ))));
        }
        if let Some(fields) = self.json.as_deref().filter(|fields| !fields.is_empty()) {
            for field in fields.split(',') {
                if !kind.fields().contains(&field) {
                    return Err(CliError::Other(anyhow::anyhow!(crate::message!(
                        "resource-json-field",
                        field = escape_controls(field),
                        fields = kind.fields().join(", ")
                    ))));
                }
            }
        }
        Pipeline::new(self.jq.as_deref(), false, false, false)?;
        Ok(())
    }

    fn project(&self, data: &Value) -> Value {
        let Some(fields) = self.json.as_deref().filter(|fields| !fields.is_empty()) else {
            return data.clone();
        };
        let select = |item: &Value| {
            let fields: Map<String, Value> = fields
                .split(',')
                .filter_map(|key| item.get(key).map(|value| (key.to_string(), value.clone())))
                .collect();
            Value::Object(fields)
        };
        match data {
            Value::Array(items) => Value::Array(items.iter().map(select).collect()),
            value => select(value),
        }
    }
}

pub fn write(
    f: &mut Factory,
    opts: &ResourceOutput,
    kind: ResourceKind,
    data: &Value,
) -> Result<(), CliError> {
    opts.validate(kind)?;
    let localizer = f.localizer.clone();
    let tty = f.io.stdout_is_tty;
    let color = f.io.color_enabled();
    let mut pager = Pager::start_localized(&mut *f.io.out, &mut *f.io.err, tty, true, &localizer);
    let result = if opts.json.is_some() {
        let projected = opts.project(data);
        let bytes = if tty {
            serde_json::to_vec_pretty(&projected)
        } else {
            serde_json::to_vec(&projected)
        }
        .map_err(anyhow::Error::from)?;
        let mut pipeline = Pipeline::new(opts.jq.as_deref(), false, color, tty)?;
        pipeline.emit_json(&mut pager, &bytes).and_then(|()| {
            if opts.jq.is_none() && !color {
                writeln!(pager)?;
            }
            pipeline.finish(&mut pager)
        })
    } else {
        render(&mut pager, &localizer, kind, data, tty, color)
    };
    let finish = pager.finish();
    result?;
    finish?;
    Ok(())
}

fn text(value: &Value, pointer: &str) -> String {
    match value.pointer(pointer) {
        Some(Value::String(s)) => cell(s),
        Some(Value::Null) | None => "-".to_string(),
        Some(value) => cell(&value.to_string()),
    }
}

pub fn cell(value: &str) -> String {
    escape_controls(value)
        .replace('\n', "\\n")
        .replace('\t', "\\t")
}

fn amount(value: &Value) -> String {
    format!(
        "{} {}",
        text(value, "/amount/total"),
        text(value, "/currency")
    )
}

fn status(value: &Value, localizer: &Localizer) -> String {
    match value.get("status").and_then(Value::as_str) {
        Some("REQUESTED") => crate::tr!(localizer, "resource-status-requested"),
        Some("SUCCEEDED") => crate::tr!(localizer, "resource-status-succeeded"),
        Some("FAILED") => crate::tr!(localizer, "resource-status-failed"),
        Some("READY") => crate::tr!(localizer, "resource-status-ready"),
        Some("PAY_PENDING" | "PENDING") => crate::tr!(localizer, "resource-status-pending"),
        Some("VIRTUAL_ACCOUNT_ISSUED") => crate::tr!(localizer, "resource-status-virtual-account"),
        Some("PAID") => crate::tr!(localizer, "resource-status-paid"),
        Some("PARTIAL_CANCELLED") => crate::tr!(localizer, "resource-status-partial-cancelled"),
        Some("CANCELLED") => crate::tr!(localizer, "resource-status-cancelled"),
        _ => text(value, "/status"),
    }
}

fn render(
    out: &mut dyn Write,
    localizer: &Localizer,
    kind: ResourceKind,
    data: &Value,
    tty: bool,
    color: bool,
) -> Result<(), CliError> {
    if let Some(items) = data.as_array() {
        if items.is_empty() {
            if tty {
                writeln!(out, "{}", crate::tr!(localizer, "resource-no-results"))?;
            }
            return Ok(());
        }
        let (headers, rows) = table_data(localizer, kind, items, tty);
        return table(out, &headers, &rows, tty, color);
    }
    match kind {
        ResourceKind::Payment | ResourceKind::Transaction => {
            payment_detail(out, localizer, data, color)
        }
        ResourceKind::Cancellation => {
            property(
                out,
                &crate::tr!(localizer, "resource-label-id"),
                &text(data, "/id"),
                color,
            )?;
            property(
                out,
                &crate::tr!(localizer, "resource-label-status"),
                &status(data, localizer),
                color,
            )?;
            property(
                out,
                &crate::tr!(localizer, "resource-label-amount"),
                &text(data, "/totalAmount"),
                color,
            )?;
            for (label, pointer) in [
                (crate::tr!(localizer, "resource-label-reason"), "/reason"),
                (
                    crate::tr!(localizer, "resource-label-requested"),
                    "/requestedAt",
                ),
                (
                    crate::tr!(localizer, "resource-label-cancelled-at"),
                    "/cancelledAt",
                ),
                (
                    crate::tr!(localizer, "resource-label-receipt"),
                    "/receiptUrl",
                ),
            ] {
                optional_property(out, &label, data, pointer, color)?;
            }
            Ok(())
        }
        ResourceKind::Webhook => {
            let (headers, rows) = table_data(localizer, kind, std::slice::from_ref(data), tty);
            table(out, &headers, &rows, tty, color)
        }
    }
}

fn table_data(
    localizer: &Localizer,
    kind: ResourceKind,
    items: &[Value],
    human: bool,
) -> (Vec<String>, Vec<Vec<String>>) {
    let id = crate::tr!(localizer, "resource-label-id");
    let status_label = crate::tr!(localizer, "resource-label-status");
    let amount_label = crate::tr!(localizer, "resource-label-amount");
    let time = crate::tr!(localizer, "resource-label-updated");
    let headers = match kind {
        ResourceKind::Payment => vec![
            id,
            status_label,
            amount_label,
            crate::tr!(localizer, "resource-label-mode"),
            crate::tr!(localizer, "resource-label-order"),
            time,
        ],
        ResourceKind::Transaction => vec![
            id,
            status_label,
            amount_label,
            crate::tr!(localizer, "resource-label-pg-tx"),
            crate::tr!(localizer, "resource-label-failure"),
            time,
        ],
        ResourceKind::Cancellation => vec![
            id,
            status_label,
            amount_label,
            crate::tr!(localizer, "resource-label-reason"),
            crate::tr!(localizer, "resource-label-requested"),
        ],
        ResourceKind::Webhook => vec![
            id,
            status_label,
            crate::tr!(localizer, "resource-label-url"),
            "HTTP".to_string(),
            crate::tr!(localizer, "resource-label-attempts"),
            crate::tr!(localizer, "resource-label-triggered"),
        ],
    };
    let rows = items
        .iter()
        .map(|v| {
            let state = if human {
                status(v, localizer)
            } else {
                text(v, "/status")
            };
            match kind {
                ResourceKind::Payment => vec![
                    text(v, "/id"),
                    state,
                    amount(v),
                    text(v, "/channel/type"),
                    text(v, "/orderName"),
                    text(v, "/statusChangedAt"),
                ],
                ResourceKind::Transaction => vec![
                    text(v, "/id"),
                    state,
                    amount(v),
                    text(v, "/pgTxId"),
                    text(v, "/failure/reason"),
                    text(v, "/statusChangedAt"),
                ],
                ResourceKind::Cancellation => vec![
                    text(v, "/id"),
                    state,
                    text(v, "/totalAmount"),
                    text(v, "/reason"),
                    text(v, "/requestedAt"),
                ],
                ResourceKind::Webhook => vec![
                    text(v, "/id"),
                    state,
                    text(v, "/url"),
                    text(v, "/response/code"),
                    format!(
                        "{}/{}",
                        text(v, "/currentExecutionCount"),
                        text(v, "/maxExecutionCount")
                    ),
                    text(v, "/triggeredAt"),
                ],
            }
        })
        .collect();
    (headers, rows)
}

fn table(
    out: &mut dyn Write,
    headers: &[String],
    rows: &[Vec<String>],
    tty: bool,
    color: bool,
) -> Result<(), CliError> {
    if !tty {
        for row in rows {
            writeln!(out, "{}", row.join("\t"))?;
        }
        return Ok(());
    }
    let widths: Vec<usize> = headers
        .iter()
        .enumerate()
        .map(|(index, header)| {
            rows.iter()
                .map(|row| row[index].width())
                .max()
                .unwrap_or(0)
                .max(header.width())
        })
        .collect();
    for (index, row) in std::iter::once(headers)
        .chain(rows.iter().map(Vec::as_slice))
        .enumerate()
    {
        if index == 0 && color {
            write!(out, "\x1b[1m")?;
        }
        for (column, value) in row.iter().enumerate() {
            write!(out, "{value}")?;
            if column + 1 < row.len() {
                write!(out, "{}", " ".repeat(widths[column] - value.width() + 2))?;
            }
        }
        if index == 0 && color {
            write!(out, "\x1b[0m")?;
        }
        writeln!(out)?;
    }
    Ok(())
}

fn property(out: &mut dyn Write, label: &str, value: &str, color: bool) -> Result<(), CliError> {
    if color {
        writeln!(out, "\x1b[1m{label}:\x1b[0m {value}")?;
    } else {
        writeln!(out, "{label}: {value}")?;
    }
    Ok(())
}

fn optional_property(
    out: &mut dyn Write,
    label: &str,
    value: &Value,
    pointer: &str,
    color: bool,
) -> Result<(), CliError> {
    if value.pointer(pointer).is_some_and(|v| !v.is_null()) {
        property(out, label, &text(value, pointer), color)?;
    }
    Ok(())
}

fn payment_detail(
    out: &mut dyn Write,
    localizer: &Localizer,
    value: &Value,
    color: bool,
) -> Result<(), CliError> {
    property(
        out,
        &crate::tr!(localizer, "resource-label-id"),
        &text(value, "/id"),
        color,
    )?;
    property(
        out,
        &crate::tr!(localizer, "resource-label-status"),
        &status(value, localizer),
        color,
    )?;
    property(
        out,
        &crate::tr!(localizer, "resource-label-amount"),
        &amount(value),
        color,
    )?;
    for (label, pointer) in [
        (crate::tr!(localizer, "resource-label-order"), "/orderName"),
        (crate::tr!(localizer, "resource-label-store"), "/storeId"),
        (crate::tr!(localizer, "resource-label-version"), "/version"),
        (
            crate::tr!(localizer, "resource-label-mode"),
            "/channel/type",
        ),
        (
            crate::tr!(localizer, "resource-label-method"),
            "/method/type",
        ),
        (
            crate::tr!(localizer, "resource-label-channel"),
            "/channel/name",
        ),
        (
            crate::tr!(localizer, "resource-label-transaction"),
            "/transactionId",
        ),
        (crate::tr!(localizer, "resource-label-pg-tx"), "/pgTxId"),
        (
            crate::tr!(localizer, "resource-label-requested"),
            "/requestedAt",
        ),
        (
            crate::tr!(localizer, "resource-label-updated"),
            "/statusChangedAt",
        ),
        (crate::tr!(localizer, "resource-label-paid"), "/paidAt"),
        (
            crate::tr!(localizer, "resource-label-cancelled-at"),
            "/cancelledAt",
        ),
        (
            crate::tr!(localizer, "resource-label-cancelled-amount"),
            "/amount/cancelled",
        ),
        (
            crate::tr!(localizer, "resource-label-failure"),
            "/failure/reason",
        ),
        (
            crate::tr!(localizer, "resource-label-pg-code"),
            "/failure/pgCode",
        ),
        (
            crate::tr!(localizer, "resource-label-pg-message"),
            "/failure/pgMessage",
        ),
        (crate::tr!(localizer, "resource-label-bank"), "/method/bank"),
        (
            crate::tr!(localizer, "resource-label-account"),
            "/method/accountNumber",
        ),
        (
            crate::tr!(localizer, "resource-label-account-holder"),
            "/method/remitteeName",
        ),
        (
            crate::tr!(localizer, "resource-label-expires"),
            "/method/expiredAt",
        ),
        (
            crate::tr!(localizer, "resource-label-receipt"),
            "/receiptUrl",
        ),
    ] {
        optional_property(out, &label, value, pointer, color)?;
    }
    for (key, title, kind) in [
        (
            "cancellations",
            crate::tr!(localizer, "resource-label-cancellations"),
            ResourceKind::Cancellation,
        ),
        (
            "webhooks",
            crate::tr!(localizer, "resource-label-webhooks"),
            ResourceKind::Webhook,
        ),
    ] {
        if let Some(items) = value
            .get(key)
            .and_then(Value::as_array)
            .filter(|items| !items.is_empty())
        {
            writeln!(out, "\n{title}")?;
            let (headers, rows) = table_data(localizer, kind, items, true);
            table(out, &headers, &rows, true, color)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn projection_keeps_nested_data_and_omits_absent_optional_fields() {
        let opts = ResourceOutput {
            json: Some("id,amount,failure".into()),
            jq: None,
        };
        let data = json!([{"id":"p","status":"PAID","amount":{"total":9_007_199_254_740_993i64}}]);
        assert_eq!(
            opts.project(&data),
            json!([{"id":"p","amount":{"total":9_007_199_254_740_993i64}}])
        );
    }

    #[test]
    fn entire_json_retains_unknown_fields_and_unknown_statuses() {
        let opts = ResourceOutput {
            json: Some(String::new()),
            jq: None,
        };
        let data = json!({"id":"p","status":"FUTURE","newField":[1,2]});
        assert_eq!(opts.project(&data), data);
    }

    #[test]
    fn validates_fields_and_jq_before_output() {
        assert!(
            ResourceOutput {
                json: Some("paymentId".into()),
                jq: None
            }
            .validate(ResourceKind::Payment)
            .is_err()
        );
        assert!(
            ResourceOutput {
                json: Some("paymentId".into()),
                jq: None
            }
            .validate(ResourceKind::Transaction)
            .is_ok()
        );
        assert!(
            ResourceOutput {
                json: Some(String::new()),
                jq: Some(".[".into())
            }
            .validate(ResourceKind::Payment)
            .is_err()
        );
    }

    #[test]
    fn non_tty_table_is_headerless_and_escapes_control_characters() {
        let mut out = Vec::new();
        render(&mut out, &Localizer::english(), ResourceKind::Payment, &json!([{"id":"p\t1","status":"PAY_PENDING","orderName":"a\nb\u{1b}","amount":{"total":123},"currency":"USD"}]), false, false).unwrap();
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "p\\t1\tPAY_PENDING\t123 USD\t-\ta\\nb\\u{1b}\t-\n"
        );
    }

    #[test]
    fn details_include_failure_and_account_fields_without_assuming_paid_shape() {
        for state in [
            "READY",
            "PAY_PENDING",
            "VIRTUAL_ACCOUNT_ISSUED",
            "PAID",
            "FAILED",
            "PARTIAL_CANCELLED",
            "CANCELLED",
        ] {
            let mut out = Vec::new();
            render(&mut out, &Localizer::english(), ResourceKind::Payment, &json!({"id":"p","status":state,"failure":{"reason":"declined"},"method":{"accountNumber":"1234"}}), false, false).unwrap();
            let out = String::from_utf8(out).unwrap();
            assert!(out.contains("declined") && out.contains("1234"), "{out}");
        }
    }
}
