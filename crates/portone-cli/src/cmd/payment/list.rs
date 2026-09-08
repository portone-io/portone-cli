use std::io::Write;

use clap::Args;
use serde_json::{Map, Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use super::{CommonArgs, array_member};
use crate::error::CliError;
use crate::factory::Factory;
use crate::http::client::Client;
use crate::output::resource::{self, ResourceKind, ResourceOutput};

const STATUSES: &[&str] = &[
    "ready",
    "pending",
    "virtual-account-issued",
    "paid",
    "failed",
    "partial-cancelled",
    "cancelled",
];
const METHODS: &[&str] = &[
    "card",
    "transfer",
    "virtual-account",
    "gift-certificate",
    "mobile",
    "easy-pay",
    "convenience-store",
    "crypto",
];
const PG_PROVIDERS: &[&str] = &[
    "html5-inicis",
    "paypal",
    "paypal-v2",
    "inicis",
    "danal",
    "nice",
    "danal-tpay",
    "jtnet",
    "uplus",
    "naverpay",
    "kakao",
    "settle",
    "kcp",
    "mobilians",
    "kakaopay",
    "naverco",
    "syrup",
    "kicc",
    "eximbay",
    "smilepay",
    "payco",
    "kcp-billing",
    "alipay",
    "payple",
    "chai",
    "bluewalnut",
    "smartro",
    "smartro-v2",
    "paymentwall",
    "tosspayments",
    "kcp-quick",
    "daou",
    "galaxia",
    "tosspay",
    "kcp-direct",
    "settle-acc",
    "settle-firm",
    "inicis-unified",
    "ksnet",
    "pinpay",
    "nice-v2",
    "toss-brandpay",
    "welcome",
    "tosspay-v2",
    "inicis-v2",
    "kpn",
    "kcp-v2",
    "hyphen",
    "eximbay-v2",
    "inicis-jp",
    "payletter-global",
    "mobilians-v2",
    "triple-a",
    "kicc-v2",
    "innopay",
    "hecto-global",
];
const SEARCH_FIELDS: &[&str] = &[
    "all",
    "payment-id",
    "tx-id",
    "schedule-id",
    "fail-reason",
    "card-issuer",
    "card-acquirer",
    "card-bin",
    "card-number",
    "card-approval-number",
    "card-receipt-name",
    "card-installment",
    "trans-bank",
    "virtual-account-holder-name",
    "virtual-account-bank",
    "virtual-account-number",
    "pg-merchant-id",
    "pg-tx-id",
    "pg-receipt-id",
    "receipt-approval-number",
    "pg-cancellation-id",
    "cancel-reason",
    "order-name",
    "customer-name",
    "customer-email",
    "customer-phone-number",
    "customer-address",
    "customer-zipcode",
    "user-agent",
    "billing-key",
    "promotion-id",
    "gift-certification-approval-number",
];
const CURRENCIES: &str = "KRW USD JPY AED AFN ALL AMD ANG AOA ARS AUD AWG AZN BAM BBD BDT BGN BHD BIF BMD BND BOB BOV BRL BSD BTN BWP BYN BZD CAD CDF CHE CHF CHW CLF CLP CNY COP COU CRC CUC CUP CVE CZK DJF DKK DOP DZD EGP ERN ETB EUR FJD FKP GBP GEL GHS GIP GMD GNF GTQ GYD HKD HNL HRK HTG HUF IDR ILS INR IQD IRR ISK JMD JOD KES KGS KHR KMF KPW KWD KYD KZT LAK LBP LKR LRD LSL LYD MAD MDL MGA MKD MMK MNT MOP MRU MUR MVR MWK MXN MXV MYR MZN NAD NGN NIO NOK NPR NZD OMR PAB PEN PGK PHP PKR PLN PYG QAR RON RSD RUB RWF SAR SBD SCR SDG SEK SGD SHP SLE SLL SOS SRD SSP STN SVC SYP SZL THB TJS TMT TND TOP TRY TTD TWD TZS UAH UGX USN UYI UYU UYW UZS VED VES VND VUV WST XAF XAG XAU XBA XBB XBC XBD XCD XDR XOF XPD XPF XPT XSU XTS XUA XXX YER ZAR ZMW ZWL";

#[derive(Debug, Args)]
pub struct ListArgs {
    #[arg(long, short = 'L', default_value_t = 30, value_parser = clap::value_parser!(u32).range(1..=60_000), help = "Maximum number of payments to fetch (1-60000)")]
    pub limit: u32,
    #[arg(long, value_delimiter = ',', value_parser = clap::builder::PossibleValuesParser::new(STATUSES), help = "Filter by payment status (repeatable or comma-separated)")]
    pub status: Vec<String>,
    #[arg(long, value_delimiter = ',', value_parser = clap::builder::PossibleValuesParser::new(METHODS), help = "Filter by payment method (repeatable or comma-separated)")]
    pub method: Vec<String>,
    #[arg(long, value_delimiter = ',', value_parser = clap::builder::PossibleValuesParser::new(PG_PROVIDERS), hide_possible_values = true, help = "Filter by PG provider (repeatable or comma-separated)")]
    pub pg: Vec<String>,
    #[arg(
        long,
        value_name = "CURRENCY",
        value_parser = clap::builder::PossibleValuesParser::new(CURRENCIES.split_whitespace()),
        ignore_case = true,
        hide_possible_values = true,
        help = "Filter by currency code, such as KRW or USD"
    )]
    pub currency: Option<String>,
    #[arg(long, conflicts_with = "live", help = "Show only test payments")]
    pub test: bool,
    #[arg(long, conflicts_with = "test", help = "Show only live payments")]
    pub live: bool,
    #[arg(long, default_value = "v2", value_parser = ["v1", "v2", "all"], help = "Filter by PortOne payment version")]
    pub version: String,
    #[arg(
        long,
        value_name = "RFC3339",
        help = "Start of the time range (default: 90 days before --until)"
    )]
    pub from: Option<String>,
    #[arg(
        long,
        value_name = "RFC3339",
        help = "End of the time range (default: now)"
    )]
    pub until: Option<String>,
    #[arg(long, default_value = "status-changed-at", value_parser = ["created-at", "status-changed-at"], help = "Timestamp used by --from and --until")]
    pub time_field: String,
    #[arg(long, default_value = "status-changed-at", value_parser = ["requested-at", "status-changed-at"], help = "Field used to sort payments")]
    pub sort: String,
    #[arg(long, default_value = "desc", value_parser = ["asc", "desc"], help = "Sort order")]
    pub order: String,
    #[arg(long, value_name = "TEXT", help = "Search payment text")]
    pub search: Option<String>,
    #[arg(long, default_value = "all", value_parser = clap::builder::PossibleValuesParser::new(SEARCH_FIELDS), hide_possible_values = true, help = "Payment field to search")]
    pub search_field: String,
    #[arg(
        long,
        conflicts_with = "store",
        help = "List all accessible stores, ignoring the default store"
    )]
    pub all_stores: bool,
    #[command(flatten)]
    pub output: ResourceOutput,
}

pub fn run(f: &mut Factory, common: &CommonArgs, args: ListArgs) -> Result<(), CliError> {
    args.output.validate(ResourceKind::Payment)?;
    if args.search.is_none() && args.search_field != "all" {
        return Err(CliError::Other(anyhow::anyhow!(crate::message!(
            "payment-search-field-requires-search"
        ))));
    }
    let store = if args.all_stores {
        None
    } else {
        common.resolve_store(f)?
    };
    let filter = build_filter(&args, store.as_deref(), OffsetDateTime::now_utc())?;
    let page_size = args.limit.min(100);
    let mut items = Vec::new();
    let mut client = Client::new(f, &common.auth())?;
    for number in 0..args.limit.div_ceil(page_size) {
        let request = json!({"page": {"number": number, "size": page_size}, "filter": filter});
        let response = client.request(&mut *f.io.err, "GET", &["payments"], &[], Some(&request))?;
        let page_items = array_member(&response, "items")?
            .as_array()
            .expect("validated array");
        let count = page_items.len();
        items.extend(
            page_items
                .iter()
                .take(args.limit as usize - items.len())
                .cloned(),
        );
        if items.len() >= args.limit as usize || count < page_size as usize {
            break;
        }
        if response
            .pointer("/page/totalCount")
            .and_then(Value::as_u64)
            .is_some_and(|total| u64::from(number + 1) * u64::from(page_size) >= total)
        {
            break;
        }
    }
    if f.io.stdout_is_tty && args.output.json.is_none() {
        let scope = store.as_deref().map(str::to_string).unwrap_or_else(|| {
            if args.all_stores {
                crate::tr!(f.localizer, "payment-list-all-stores")
            } else {
                crate::tr!(f.localizer, "payment-list-api-default")
            }
        });
        let environment = if args.test {
            "TEST"
        } else if args.live {
            "LIVE"
        } else {
            "TEST,LIVE"
        };
        let scope = crate::tr!(
            f.localizer,
            "payment-list-scope",
            store = resource::cell(&scope),
            version = args.version.to_ascii_uppercase(),
            environment = environment,
            from = filter["from"].as_str().unwrap_or_default(),
            until = filter["until"].as_str().unwrap_or_default()
        );
        writeln!(f.io.err, "{}", crate::output::escape_controls(&scope))?;
    }
    resource::write(f, &args.output, ResourceKind::Payment, &Value::Array(items))
}

fn build_filter(
    args: &ListArgs,
    store: Option<&str>,
    now: OffsetDateTime,
) -> Result<Value, CliError> {
    let until = args
        .until
        .as_deref()
        .map(|v| parse_date(v, "--until"))
        .transpose()?
        .unwrap_or(now);
    let from = match args.from.as_deref() {
        Some(value) => parse_date(value, "--from")?,
        None => until.checked_sub(time::Duration::days(90)).ok_or_else(|| {
            CliError::Other(anyhow::anyhow!(crate::message!("payment-date-range")))
        })?,
    };
    if from > until {
        return Err(CliError::Other(anyhow::anyhow!(crate::message!(
            "payment-date-range"
        ))));
    }
    let mut filter = Map::new();
    filter.insert(
        "from".into(),
        json!(from.format(&Rfc3339).map_err(anyhow::Error::from)?),
    );
    filter.insert(
        "until".into(),
        json!(until.format(&Rfc3339).map_err(anyhow::Error::from)?),
    );
    filter.insert("timestampType".into(), json!(api_enum(&args.time_field)));
    filter.insert("sortBy".into(), json!(api_enum(&args.sort)));
    filter.insert("sortOrder".into(), json!(api_enum(&args.order)));
    if let Some(store) = store {
        filter.insert("storeId".into(), json!(store));
    }
    for (key, values) in [
        ("status", &args.status),
        ("methods", &args.method),
        ("pgProvider", &args.pg),
    ] {
        if !values.is_empty() {
            filter.insert(
                key.into(),
                json!(values.iter().map(|s| api_enum(s)).collect::<Vec<_>>()),
            );
        }
    }
    if args.version != "all" {
        filter.insert("version".into(), json!(api_enum(&args.version)));
    }
    if args.test || args.live {
        filter.insert("isTest".into(), json!(args.test));
    }
    if let Some(currency) = &args.currency {
        super::nonempty(currency, "--currency")?;
        filter.insert("currency".into(), json!(currency.to_ascii_uppercase()));
    }
    if let Some(search) = &args.search {
        super::nonempty(search, "--search")?;
        filter.insert(
            "textSearch".into(),
            json!([{"field": api_enum(&args.search_field), "value": search}]),
        );
    }
    Ok(Value::Object(filter))
}

fn api_enum(value: &str) -> String {
    value.replace('-', "_").to_ascii_uppercase()
}

fn parse_date(value: &str, field: &str) -> Result<OffsetDateTime, CliError> {
    OffsetDateTime::parse(value, &Rfc3339).map_err(|_| {
        CliError::Other(anyhow::anyhow!(crate::message!(
            "payment-date-invalid",
            field = field,
            value = resource::cell(value)
        )))
    })
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

    fn parse(args: &[&str]) -> ListArgs {
        let cli = TestCli::try_parse_from(std::iter::once("payment").chain(args.iter().copied()))
            .unwrap();
        match cli.payment.command {
            PaymentCommand::List(args) => args,
            _ => panic!("not list"),
        }
    }

    #[test]
    fn filter_defaults_are_explicit_and_stable() {
        let args = parse(&["list"]);
        let now = parse_date("2026-09-08T12:00:00Z", "now").unwrap();
        let filter = build_filter(&args, Some("store"), now).unwrap();
        assert_eq!(filter["version"], "V2");
        assert_eq!(filter["timestampType"], "STATUS_CHANGED_AT");
        assert_eq!(filter["sortBy"], "STATUS_CHANGED_AT");
        assert_eq!(filter["sortOrder"], "DESC");
        assert_eq!(filter["from"], "2026-06-10T12:00:00Z");
        assert_eq!(filter["until"], "2026-09-08T12:00:00Z");
        assert_eq!(filter["storeId"], "store");
        assert!(filter.get("isTest").is_none());
        assert!(filter.get("status").is_none());
    }

    #[test]
    fn maps_repeatable_filters_and_literal_search() {
        let args = parse(&[
            "list",
            "--status",
            "pending,paid",
            "--status",
            "failed",
            "--method",
            "virtual-account",
            "--pg",
            "nice-v2",
            "--version",
            "all",
            "--live",
            "--search",
            "foo:bar",
            "--search-field",
            "order-name",
        ]);
        let filter = build_filter(&args, None, OffsetDateTime::now_utc()).unwrap();
        assert_eq!(filter["status"], json!(["PENDING", "PAID", "FAILED"]));
        assert_eq!(filter["methods"], json!(["VIRTUAL_ACCOUNT"]));
        assert_eq!(filter["pgProvider"], json!(["NICE_V2"]));
        assert_eq!(
            filter["textSearch"],
            json!([{"field":"ORDER_NAME","value":"foo:bar"}])
        );
        assert_eq!(filter["isTest"], false);
        assert!(filter.get("version").is_none());
    }

    #[test]
    fn validates_dates_with_offsets_and_reversed_ranges() {
        let args = parse(&[
            "list",
            "--from",
            "2026-09-08T09:00:00+09:00",
            "--until",
            "2026-09-08T01:00:00Z",
        ]);
        assert!(build_filter(&args, None, OffsetDateTime::now_utc()).is_ok());
        let args = parse(&[
            "list",
            "--from",
            "2026-09-08T09:00:00Z",
            "--until",
            "2026-09-08T01:00:00Z",
        ]);
        assert!(build_filter(&args, None, OffsetDateTime::now_utc()).is_err());
        assert!(parse_date("2026-09-08", "--from").is_err());
    }

    #[test]
    fn rejects_invalid_limits_and_conflicting_scope() {
        for args in [
            vec!["payment", "list", "--limit", "0"],
            vec!["payment", "list", "--limit", "60001"],
            vec!["payment", "list", "--test", "--live"],
            vec!["payment", "list", "--all-stores", "--store", "id"],
        ] {
            assert!(TestCli::try_parse_from(args).is_err());
        }
    }

    #[test]
    fn currency_uses_schema_codes_and_accepts_lowercase() {
        let args = parse(&["list", "--currency", "krw"]);
        let filter = build_filter(&args, None, OffsetDateTime::now_utc()).unwrap();
        assert_eq!(filter["currency"], "KRW");
        assert!(TestCli::try_parse_from(["payment", "list", "--currency", "INVALID"]).is_err());
    }
}
