//! Import named string enums from the committed PortOne OpenAPI schema.

mod expand;

use std::sync::LazyLock;

use proc_macro::TokenStream;
use serde_json::Value;

// Embedding in the macro crate makes schema edits a compiler-tracked dependency.
// No filesystem or network access is performed during macro expansion.
static SCHEMA: LazyLock<Result<Value, serde_json::Error>> =
    LazyLock::new(|| serde_json::from_str(include_str!("../schema/v2.openapi.json")));

/// Generate a named string enum with clap value parsing and serde serialization.
///
/// ```
/// use portone_schema_macros::schema_enum;
///
/// schema_enum!(pub PaymentStatus);
/// schema_enum!(pub Currency, cli_case = "preserve");
///
/// assert_eq!(PaymentStatus::Paid.as_api_str(), "PAID");
/// assert_eq!(Currency::Krw.as_api_str(), "KRW");
/// ```
///
/// CLI values default to lowercase kebab-case. `cli_case = "preserve"` keeps
/// the schema spelling. API serialization always preserves the original value.
#[proc_macro]
pub fn schema_enum(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as expand::Input);
    let result = match &*SCHEMA {
        Ok(schema) => expand::generate(schema, &input),
        Err(error) => Err(syn::Error::new(
            input.name.span(),
            format!("invalid embedded OpenAPI schema: {error}"),
        )),
    };
    result.unwrap_or_else(syn::Error::into_compile_error).into()
}
