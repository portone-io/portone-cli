use std::collections::HashSet;

use heck::ToUpperCamelCase;
use proc_macro2::TokenStream;
use quote::quote;
use serde_json::Value;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitStr, Token, Visibility};

pub(super) struct Input {
    visibility: Visibility,
    pub(super) name: Ident,
    preserve_case: bool,
}

impl Parse for Input {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let visibility = input.parse()?;
        let name = input.parse()?;
        let mut preserve_case = false;
        if !input.is_empty() {
            input.parse::<Token![,]>()?;
            if !input.is_empty() {
                let key: Ident = input.parse()?;
                if key != "cli_case" {
                    return Err(syn::Error::new(key.span(), "expected `cli_case`"));
                }
                input.parse::<Token![=]>()?;
                let value: LitStr = input.parse()?;
                preserve_case = match value.value().as_str() {
                    "kebab-case" => false,
                    "preserve" => true,
                    _ => {
                        return Err(syn::Error::new(
                            value.span(),
                            "expected `kebab-case` or `preserve`",
                        ));
                    }
                };
                if !input.is_empty() {
                    input.parse::<Token![,]>()?;
                }
            }
        }
        Ok(Self {
            visibility,
            name,
            preserve_case,
        })
    }
}

pub(super) fn generate(schema: &Value, input: &Input) -> syn::Result<TokenStream> {
    let Input {
        visibility,
        name,
        preserve_case,
    } = input;
    let type_name = name.to_string();
    let error = |message: String| syn::Error::new(name.span(), message);
    let definition = schema
        .get("components")
        .and_then(|value| value.get("schemas"))
        .and_then(|value| value.get(&type_name))
        .ok_or_else(|| error(format!("schema type `{type_name}` does not exist")))?;
    if definition.get("type").and_then(Value::as_str) != Some("string")
        || ["$ref", "allOf", "anyOf", "oneOf"]
            .iter()
            .any(|key| definition.get(key).is_some())
    {
        return Err(error(format!(
            "schema type `{type_name}` must be a direct string enum"
        )));
    }
    let values = definition
        .get("enum")
        .and_then(Value::as_array)
        .filter(|values| !values.is_empty())
        .ok_or_else(|| {
            error(format!(
                "schema type `{type_name}` must have a nonempty enum array"
            ))
        })?;

    let mut wire_names = HashSet::new();
    let mut rust_names = HashSet::new();
    let mut cli_names = HashSet::new();
    let mut variants = Vec::new();
    let mut api_arms = Vec::new();
    for value in values {
        let wire = value
            .as_str()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                error(format!(
                    "schema type `{type_name}` has an empty or non-string enum value"
                ))
            })?;
        if !wire_names.insert(wire) {
            return Err(error(format!(
                "schema type `{type_name}` has duplicate value `{wire}`"
            )));
        }
        let rust_name = wire.to_upper_camel_case();
        let mut variant = syn::parse_str::<Ident>(&rust_name).map_err(|_| {
            error(format!(
                "schema value `{type_name}.{wire}` cannot form a Rust variant"
            ))
        })?;
        // These are path keywords, even though syn accepts some of them as identifiers.
        if rust_name == "Self" {
            return Err(error(format!(
                "schema value `{type_name}.{wire}` cannot form a Rust variant"
            )));
        }
        variant.set_span(name.span());
        if !rust_names.insert(rust_name.clone()) {
            return Err(error(format!(
                "schema type `{type_name}` has conflicting Rust variant `{rust_name}`"
            )));
        }
        let cli = if *preserve_case {
            wire.to_owned()
        } else {
            wire.replace('_', "-").to_ascii_lowercase()
        };
        if !cli_names.insert(cli.clone()) {
            return Err(error(format!(
                "schema type `{type_name}` has conflicting CLI value `{cli}`"
            )));
        }
        variants.push(quote! {
            #[value(name = #cli)]
            #[serde(rename = #wire)]
            #variant,
        });
        api_arms.push(quote! { Self::#variant => #wire, });
    }
    Ok(quote! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, ::clap::ValueEnum, ::serde::Serialize)]
        #visibility enum #name { #(#variants)* }

        impl #name {
            /// Return the exact value defined by the OpenAPI schema.
            pub const fn as_api_str(&self) -> &'static str {
                match self { #(#api_arms)* }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn every_named_string_enum_can_generate_valid_rust() {
        let schema = crate::SCHEMA.as_ref().unwrap();
        let definitions = schema["components"]["schemas"].as_object().unwrap();
        let mut count = 0;
        for (name, definition) in definitions {
            if definition["type"] != "string" || definition.get("enum").is_none() {
                continue;
            }
            let input = syn::parse_str(&format!("pub {name}")).unwrap();
            let tokens = generate(schema, &input).unwrap_or_else(|error| panic!("{name}: {error}"));
            syn::parse2::<syn::File>(tokens).unwrap();
            count += 1;
        }
        assert!(count > 0);
    }

    #[test]
    fn invalid_definitions_report_the_schema_type() {
        for (definition, expected) in [
            (json!({"type":"object"}), "direct string enum"),
            (json!({"type":"string", "enum":[]}), "nonempty enum array"),
            (
                json!({"type":"string", "enum":[1]}),
                "non-string enum value",
            ),
            (json!({"type":"string", "enum":[""]}), "empty or non-string"),
            (
                json!({"type":"string", "enum":["A", "A"]}),
                "duplicate value",
            ),
            (
                json!({"type":"string", "enum":["A_B", "A-B"]}),
                "conflicting Rust variant",
            ),
            (
                json!({"type":"string", "enum":["123"]}),
                "cannot form a Rust variant",
            ),
            (
                json!({"type":"string", "enum":["SELF"]}),
                "cannot form a Rust variant",
            ),
            (
                json!({"type":"string", "enum":["A"], "$ref":"elsewhere"}),
                "direct string enum",
            ),
        ] {
            let schema = json!({"components":{"schemas":{"Example":definition}}});
            let input = syn::parse_str("pub Example").unwrap();
            let error = generate(&schema, &input).unwrap_err().to_string();
            assert!(
                error.contains("Example") && error.contains(expected),
                "{error}"
            );
        }
    }
}
