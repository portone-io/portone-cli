use clap::{Parser, ValueEnum};
use portone_schema_macros::schema_enum;

schema_enum!(pub PaymentStatus);
schema_enum!(pub PgProvider);
schema_enum!(pub Currency, cli_case = "preserve");

#[derive(Parser)]
struct Args {
    #[arg(long, value_enum, value_delimiter = ',')]
    status: Vec<PaymentStatus>,
    #[arg(long, value_enum, ignore_case = true)]
    currency: Currency,
}

#[test]
fn cli_spelling_is_independent_of_api_serialization() {
    let args = Args::try_parse_from([
        "test",
        "--status",
        "paid,partial-cancelled",
        "--status",
        "failed",
        "--currency",
        "krw",
    ])
    .unwrap();
    assert_eq!(
        serde_json::to_value(args.status).unwrap(),
        serde_json::json!(["PAID", "PARTIAL_CANCELLED", "FAILED"])
    );
    assert_eq!(serde_json::to_value(args.currency).unwrap(), "KRW");
    assert_eq!(args.currency.to_possible_value().unwrap().get_name(), "KRW");
    assert_eq!(
        PgProvider::from_str("html5-inicis", false)
            .unwrap()
            .as_api_str(),
        "HTML5_INICIS"
    );
    assert!(Args::try_parse_from(["test", "--currency", "unknown"]).is_err());
    assert!(Args::try_parse_from(["test", "--currency", "KRW", "--status", "PAID"]).is_err());
}
