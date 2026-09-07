use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Barrier};

use fluent_syntax::{ast, parser};

use super::*;

type PreferenceCase<'a> = (&'a str, &'a [(&'a str, &'a str)], &'a [&'a str], Language);

fn preferences(values: &[(&str, &str)], system_languages: &[&str]) -> LanguagePreferences {
    let mut preferences = LanguagePreferences {
        system_languages: system_languages
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        ..LanguagePreferences::default()
    };
    for (key, value) in values {
        let field = match *key {
            "PORTONE_LANG" => &mut preferences.portone_lang,
            "config" => &mut preferences.configured,
            other => panic!("unknown preference {other}"),
        };
        *field = Some((*value).to_string());
    }
    preferences
}

#[test]
fn explicit_preferences_and_auto_have_defined_precedence() {
    use Language::{English, Korean};
    let cases: &[PreferenceCase<'_>] = &[
        ("unset", &[], &[], English),
        ("configured Korean", &[("config", "ko")], &["en-US"], Korean),
        (
            "configured English",
            &[("config", "en")],
            &["ko-KR"],
            English,
        ),
        (
            "environment overrides config and system",
            &[("PORTONE_LANG", "en"), ("config", "ko")],
            &["ko-KR"],
            English,
        ),
        (
            "environment overrides system",
            &[("PORTONE_LANG", "ko")],
            &["en-US"],
            Korean,
        ),
        (
            "auto skips stored English",
            &[("PORTONE_LANG", "auto"), ("config", "en")],
            &["ko-KR"],
            Korean,
        ),
        (
            "auto skips stored Korean",
            &[("PORTONE_LANG", "auto"), ("config", "ko")],
            &["en-US"],
            English,
        ),
        ("configured auto", &[("config", "auto")], &["ko-KR"], Korean),
        (
            "auto is case insensitive",
            &[("PORTONE_LANG", " AUTO "), ("config", "en")],
            &["ko-KR"],
            Korean,
        ),
        (
            "empty environment uses config",
            &[("PORTONE_LANG", "  "), ("config", "ko")],
            &["en-US"],
            Korean,
        ),
        (
            "empty config uses system",
            &[("config", "  ")],
            &["ko-KR"],
            Korean,
        ),
        (
            "unknown environment falls back",
            &[("PORTONE_LANG", "fr"), ("config", "ko")],
            &["ko-KR"],
            English,
        ),
        (
            "unknown config falls back",
            &[("config", "ja")],
            &["ko-KR"],
            English,
        ),
        (
            "malformed configured value falls back",
            &[("config", "!invalid!")],
            &["ko-KR"],
            English,
        ),
        (
            "trimmed explicit regional language",
            &[("PORTONE_LANG", " KO-kr.UTF-8 ")],
            &["en-US"],
            Korean,
        ),
    ];
    for (description, values, system_languages, expected) in cases {
        assert_eq!(
            preferences(values, system_languages).resolve(),
            *expected,
            "{description}"
        );
    }
}

#[test]
fn system_languages_select_the_first_supported_language() {
    use Language::{English, Korean};
    for (languages, expected) in [
        (vec!["ja-JP", "ko-KR", "en-US"], Korean),
        (vec!["en-US", "ko-KR"], English),
        (vec!["ko-KR", "en-US"], Korean),
        (vec!["ja-JP", "fr-FR"], English),
        (vec!["", "!invalid!", "ko-KR"], Korean),
        (vec!["C", "POSIX", "ko-KR"], Korean),
        (vec![], English),
    ] {
        assert_eq!(
            preferences(&[], &languages).resolve(),
            expected,
            "{languages:?}"
        );
    }
}

#[test]
fn locale_normalization_accepts_supported_variants() {
    for value in [
        "ko",
        "ko_KR",
        "ko-KR",
        "ko_KR.UTF-8",
        "KO-kr",
        " ko_KR.UTF-8@modifier ",
    ] {
        assert_eq!(supported(value), Some(Language::Korean), "{value}");
    }
    for value in ["en", "en-US", "en_GB.UTF-8"] {
        assert_eq!(supported(value), Some(Language::English), "{value}");
    }
    for value in [
        "",
        "ja_JP.UTF-8",
        "fr-FR",
        "korean",
        "english",
        "C",
        "C.UTF-8",
        "POSIX",
    ] {
        assert_eq!(supported(value), None, "{value}");
    }
}

#[test]
fn simultaneous_localizers_do_not_change_each_other_or_error_display() {
    let english = Arc::new(Localizer::english());
    let korean = Arc::new(Localizer::korean());
    let barrier = Arc::new(Barrier::new(8));
    let workers: Vec<_> = (0..8)
        .map(|index| {
            let localizer = if index % 2 == 0 {
                english.clone()
            } else {
                korean.clone()
            };
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                let expected = if index % 2 == 0 {
                    "Authentication: Console OAuth"
                } else {
                    "인증 방식: 콘솔 OAuth"
                };
                for _ in 0..16 {
                    assert_eq!(
                        crate::tr!(localizer, "auth-status-authentication"),
                        expected
                    );
                    assert_eq!(
                        crate::message!("auth-token-read-failed").to_string(),
                        "failed to read token response"
                    );
                }
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
    assert_eq!(english.lang(), "en");
    assert_eq!(korean.lang(), "ko");
}

struct FallbackFixture;

impl I18nAssets for FallbackFixture {
    fn get_files(&self, path: &str) -> Vec<Cow<'_, [u8]>> {
        let text = match path {
            "en-US/fixture.ftl" => {
                "translated = English text\nfallback-only = English fallback for { $name }\n"
            }
            "ko/fixture.ftl" => "translated = 한국어 문구\n",
            _ => return Vec::new(),
        };
        vec![Cow::Borrowed(text.as_bytes())]
    }

    fn filenames_iter(&self) -> Box<dyn Iterator<Item = String> + '_> {
        Box::new(
            ["en-US/fixture.ftl", "ko/fixture.ftl"]
                .into_iter()
                .map(str::to_string),
        )
    }
}

#[test]
fn missing_korean_translation_uses_english_fallback() {
    let loader = FluentLanguageLoader::new("fixture", "en-US".parse().unwrap());
    i18n_embed::select(&loader, &FallbackFixture, &["ko".parse().unwrap()]).unwrap();
    loader.set_use_isolating(false);
    assert_eq!(loader.get("translated"), "한국어 문구");
    assert_eq!(
        loader.get_args("fallback-only", HashMap::from([("name", "merchant-42")])),
        "English fallback for merchant-42"
    );
}

#[test]
fn localized_contexts_preserve_external_details_and_broken_pipe_identity() {
    let source = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "external diagnostic");
    let error = Err::<(), _>(source)
        .with_lcontext(|| crate::message!("auth-token-read-failed"))
        .unwrap_err()
        .lcontext(crate::message!("auth-login-token-failed"));
    assert_eq!(
        Localizer::english().format_error(&error),
        "failed to obtain tokens: failed to read token response: external diagnostic"
    );
    assert_eq!(
        Localizer::korean().format_error(&error),
        "토큰을 받지 못했습니다: 토큰 응답을 읽지 못했습니다: external diagnostic"
    );
    assert!(error.chain().any(|cause| {
        cause
            .downcast_ref::<std::io::Error>()
            .is_some_and(|source| source.kind() == std::io::ErrorKind::BrokenPipe)
    }));
    let message = anyhow::anyhow!(crate::message!("auth-token-missing-access-token"));
    assert_eq!(
        Localizer::korean().format_error(&message),
        "토큰 응답에 access_token이 없습니다"
    );
    assert_eq!(
        Ok::<_, std::io::Error>(42)
            .with_lcontext(|| panic!("successful results must not build error contexts"))
            .unwrap(),
        42
    );
}

#[derive(Default)]
struct Usage {
    variables: BTreeSet<String>,
    references: BTreeSet<String>,
}

fn inspect_arguments(arguments: &ast::CallArguments<&str>, usage: &mut Usage) {
    for argument in &arguments.positional {
        inspect_inline(argument, usage);
    }
    for argument in &arguments.named {
        inspect_inline(&argument.value, usage);
    }
}

fn reference_name(name: &str, attribute: Option<&ast::Identifier<&str>>) -> String {
    match attribute {
        Some(attribute) => format!("{name}.{}", attribute.name),
        None => name.to_string(),
    }
}

fn inspect_inline(expression: &ast::InlineExpression<&str>, usage: &mut Usage) {
    match expression {
        ast::InlineExpression::VariableReference { id } => {
            usage.variables.insert(id.name.to_string());
        }
        ast::InlineExpression::MessageReference { id, attribute } => {
            usage
                .references
                .insert(reference_name(id.name, attribute.as_ref()));
        }
        ast::InlineExpression::TermReference {
            id,
            attribute,
            arguments,
        } => {
            usage
                .references
                .insert(reference_name(&format!("-{}", id.name), attribute.as_ref()));
            if let Some(arguments) = arguments {
                inspect_arguments(arguments, usage);
            }
        }
        ast::InlineExpression::FunctionReference { arguments, .. } => {
            inspect_arguments(arguments, usage)
        }
        ast::InlineExpression::Placeable { expression } => inspect_expression(expression, usage),
        ast::InlineExpression::StringLiteral { .. }
        | ast::InlineExpression::NumberLiteral { .. } => {}
    }
}

fn inspect_expression(expression: &ast::Expression<&str>, usage: &mut Usage) {
    match expression {
        ast::Expression::Inline(inline) => inspect_inline(inline, usage),
        ast::Expression::Select { selector, variants } => {
            inspect_inline(selector, usage);
            for variant in variants {
                inspect_pattern(&variant.value, usage);
            }
        }
    }
}

fn inspect_pattern(pattern: &ast::Pattern<&str>, usage: &mut Usage) {
    for element in &pattern.elements {
        if let ast::PatternElement::Placeable { expression } = element {
            inspect_expression(expression, usage);
        }
    }
}

struct Template<'a> {
    pattern: ast::Pattern<&'a str>,
    usage: Usage,
}

struct Catalog<'a> {
    entries: BTreeSet<String>,
    templates: BTreeMap<String, Template<'a>>,
}

fn parse_catalog<'a>(text: &'a str, locale: &str) -> Catalog<'a> {
    let resource = parser::parse(text)
        .unwrap_or_else(|(_, errors)| panic!("{locale}: invalid Fluent syntax: {errors:?}"));
    let mut catalog = Catalog {
        entries: BTreeSet::new(),
        templates: BTreeMap::new(),
    };
    for entry in resource.body {
        let (name, value, attributes) = match entry {
            ast::Entry::Message(message) => (
                message.id.name.to_string(),
                message.value,
                message.attributes,
            ),
            ast::Entry::Term(term) => (
                format!("-{}", term.id.name),
                Some(term.value),
                term.attributes,
            ),
            ast::Entry::Comment(_)
            | ast::Entry::GroupComment(_)
            | ast::Entry::ResourceComment(_) => continue,
            ast::Entry::Junk { content } => panic!("{locale}: invalid Fluent content: {content}"),
        };
        assert!(
            catalog.entries.insert(name.clone()),
            "{locale}: duplicate entry {name}"
        );
        let patterns = value.into_iter().map(|value| (name.clone(), value)).chain(
            attributes
                .into_iter()
                .map(|attribute| (format!("{name}.{}", attribute.id.name), attribute.value)),
        );
        for (name, pattern) in patterns {
            let mut usage = Usage::default();
            inspect_pattern(&pattern, &mut usage);
            assert!(
                catalog
                    .templates
                    .insert(name.clone(), Template { pattern, usage })
                    .is_none(),
                "{locale}: duplicate template {name}"
            );
        }
    }
    for (name, template) in &catalog.templates {
        for reference in &template.usage.references {
            assert!(
                catalog.templates.contains_key(reference),
                "{locale}: {name} references missing {reference}"
            );
        }
    }
    catalog
}

#[test]
fn embedded_catalogs_have_matching_keys_variables_and_valid_references() {
    let read = |language| {
        CATALOGS
            .get_file(format!("{language}/portone-cli.ftl"))
            .unwrap()
            .contents_utf8()
            .unwrap()
    };
    let english = parse_catalog(read("en-US"), "en-US");
    let korean = parse_catalog(read("ko"), "ko");
    assert!(!english.entries.is_empty());
    assert_eq!(
        english.entries, korean.entries,
        "entry keys must match between catalogs"
    );
    assert_eq!(
        english.templates.keys().collect::<Vec<_>>(),
        korean.templates.keys().collect::<Vec<_>>(),
        "message values and attributes must match between catalogs"
    );
    for (name, template) in &english.templates {
        assert_eq!(
            template.usage.variables, korean.templates[name].usage.variables,
            "{name}: interpolation variables differ between languages"
        );
    }
    for (localizer, catalog) in [
        (Localizer::english(), english),
        (Localizer::korean(), korean),
    ] {
        let anchor = catalog
            .entries
            .iter()
            .find(|name| !name.starts_with('-'))
            .unwrap();
        let variables: BTreeSet<_> = catalog
            .templates
            .values()
            .flat_map(|template| template.usage.variables.iter())
            .collect();
        localizer
            .loader()
            .with_fluent_message_and_bundle(anchor, |_, bundle| {
                let args = variables
                    .iter()
                    .map(|name| (name.as_str(), "sample"))
                    .collect();
                for (name, template) in &catalog.templates {
                    let mut errors = Vec::new();
                    let rendered =
                        bundle.format_pattern(&template.pattern, Some(&args), &mut errors);
                    assert!(
                        errors.is_empty(),
                        "{} {name}: {errors:?}; output: {rendered}",
                        localizer.lang()
                    );
                    assert!(
                        !rendered.contains(['\u{2068}', '\u{2069}']),
                        "{} {name}: hidden bidi isolation in terminal output",
                        localizer.lang()
                    );
                }
            })
            .expect("catalog has at least one message");
    }
}
