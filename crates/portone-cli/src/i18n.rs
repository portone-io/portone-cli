//! Invocation-scoped localization. Protocol data and external errors are never translated.
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;
use std::sync::OnceLock;

use i18n_embed::fluent::{FluentLanguageLoader, fluent_language_loader};
use i18n_embed::{DesktopLanguageRequester, I18nAssets};
use include_dir::{Dir, include_dir};

static CATALOGS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/i18n");

struct Catalogs;

impl I18nAssets for Catalogs {
    fn get_files(&self, path: &str) -> Vec<Cow<'_, [u8]>> {
        CATALOGS
            .get_file(path)
            .map(|file| Cow::Borrowed(file.contents()))
            .into_iter()
            .collect()
    }

    fn filenames_iter(&self) -> Box<dyn Iterator<Item = String> + '_> {
        fn walk(dir: &Dir<'_>, names: &mut Vec<String>) {
            names.extend(
                dir.files()
                    .map(|file| file.path().to_string_lossy().replace('\\', "/")),
            );
            for child in dir.dirs() {
                walk(child, names);
            }
        }
        let mut names = Vec::new();
        walk(&CATALOGS, &mut names);
        Box::new(names.into_iter())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    English,
    Korean,
}

impl Language {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Korean => "ko",
        }
    }
}

/// Inputs are separated from process state so resolution can be tested without
/// changing the environment of concurrently running tests.
#[derive(Debug, Default)]
pub struct LanguagePreferences {
    pub portone_lang: Option<String>,
    pub configured: Option<String>,
    pub system_languages: Vec<String>,
}

fn nonempty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn normalized(value: &str) -> String {
    value
        .trim()
        .split(['.', '@'])
        .next()
        .unwrap_or_default()
        .replace('_', "-")
        .to_ascii_lowercase()
}

fn supported(value: &str) -> Option<Language> {
    match normalized(value).split('-').next()? {
        "en" => Some(Language::English),
        "ko" => Some(Language::Korean),
        _ => None,
    }
}

impl LanguagePreferences {
    pub fn resolve(&self) -> Language {
        let explicit =
            nonempty(self.portone_lang.as_deref()).or_else(|| nonempty(self.configured.as_deref()));
        if let Some(value) = explicit
            && !value.eq_ignore_ascii_case("auto")
        {
            return supported(value).unwrap_or(Language::English);
        }

        self.system_languages
            .iter()
            .find_map(|value| supported(value))
            .unwrap_or(Language::English)
    }

    fn detect() -> Self {
        let portone_lang = std::env::var("PORTONE_LANG").ok();
        let configured = if nonempty(portone_lang.as_deref()).is_none() {
            read_configured_language()
        } else {
            None
        };
        Self {
            portone_lang,
            configured,
            // sys-locale owns platform-specific detection, including native UI
            // preferences on macOS and Windows regardless of POSIX locale variables.
            system_languages: DesktopLanguageRequester::requested_languages()
                .iter()
                .map(ToString::to_string)
                .collect(),
        }
    }
}

fn read_configured_language() -> Option<String> {
    #[derive(serde::Deserialize)]
    struct Preference {
        language: Option<String>,
    }
    // Help/version must work even when home discovery or the full config fails.
    let path = crate::config::paths::try_config_dir()?.join("config.toml");
    let text = std::fs::read_to_string(path).ok()?;
    toml::from_str::<Preference>(&text).ok()?.language
}

#[derive(Debug)]
pub struct Localizer {
    language: Language,
    loader: FluentLanguageLoader,
}

impl Localizer {
    pub fn new(language: Language) -> Self {
        let loader = fluent_language_loader!();
        let requested = match language {
            Language::English => "en-US",
            Language::Korean => "ko",
        }
        .parse()
        .expect("built-in language identifier");
        i18n_embed::select(&loader, &Catalogs, &[requested])
            .expect("validated embedded translations");
        loader.set_use_isolating(false);
        Self { language, loader }
    }

    pub fn detect() -> Self {
        Self::new(LanguagePreferences::detect().resolve())
    }

    pub fn english() -> Self {
        Self::new(Language::English)
    }
    pub fn korean() -> Self {
        Self::new(Language::Korean)
    }
    pub fn lang(&self) -> &'static str {
        self.language.as_str()
    }
    pub fn loader(&self) -> &FluentLanguageLoader {
        &self.loader
    }

    pub fn format_error(&self, error: &anyhow::Error) -> String {
        error
            .chain()
            .map(|cause| {
                cause
                    .downcast_ref::<LocalizedMessage>()
                    .map(|message| message.render(self))
                    .or_else(|| {
                        cause
                            .downcast_ref::<MessageContext>()
                            .map(|context| context.message.render(self))
                    })
                    .unwrap_or_else(|| cause.to_string())
            })
            .collect::<Vec<_>>()
            .join(": ")
    }
}

/// A fixed English context for Display and tests; it never follows or changes
/// the process's selected language.
#[doc(hidden)]
pub fn english() -> &'static Localizer {
    static ENGLISH: OnceLock<Localizer> = OnceLock::new();
    ENGLISH.get_or_init(Localizer::english)
}

/// Keep message identity through anyhow's error chain until the UI renders it.
/// This also leaves source errors available for broken-pipe and other checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalizedMessage {
    id: &'static str,
    args: HashMap<&'static str, String>,
    english: String,
}

impl LocalizedMessage {
    #[doc(hidden)]
    pub fn new(id: &'static str, args: HashMap<&'static str, String>, english: String) -> Self {
        Self { id, args, english }
    }

    pub fn render(&self, localizer: &Localizer) -> String {
        localizer.loader.get_args(self.id, self.args.clone())
    }
}

impl fmt::Display for LocalizedMessage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.english)
    }
}

impl std::error::Error for LocalizedMessage {}

#[derive(Debug)]
struct MessageContext {
    message: LocalizedMessage,
    source: anyhow::Error,
}

impl fmt::Display for MessageContext {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message.fmt(formatter)
    }
}

impl std::error::Error for MessageContext {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.source.as_ref())
    }
}

/// Unlike anyhow's opaque context wrapper, this retains the message's identity
/// in the standard error chain while preserving the underlying error.
pub trait LocalizedContext<T> {
    fn lcontext(self, message: LocalizedMessage) -> anyhow::Result<T>;
    fn with_lcontext(self, message: impl FnOnce() -> LocalizedMessage) -> anyhow::Result<T>;
}

impl<T, E: Into<anyhow::Error>> LocalizedContext<T> for Result<T, E> {
    fn lcontext(self, message: LocalizedMessage) -> anyhow::Result<T> {
        self.with_lcontext(|| message)
    }

    fn with_lcontext(self, message: impl FnOnce() -> LocalizedMessage) -> anyhow::Result<T> {
        self.map_err(|source| {
            anyhow::Error::new(MessageContext {
                message: message(),
                source: source.into(),
            })
        })
    }
}

pub trait LocalizedErrorContext {
    fn lcontext(self, message: LocalizedMessage) -> anyhow::Error;
}

impl LocalizedErrorContext for anyhow::Error {
    fn lcontext(self, message: LocalizedMessage) -> anyhow::Error {
        anyhow::Error::new(MessageContext {
            message,
            source: self,
        })
    }
}

/// Translate at the presentation boundary, checking keys/arguments at compile time.
#[macro_export]
macro_rules! tr {
    ($localizer:expr, $id:literal $(, $name:ident = $value:expr)* $(,)?) => {
        i18n_embed_fl::fl!(($localizer).loader(), $id $(, $name = ($value))*)
    };
}

/// Construct a translatable error without global mutable locale state.
#[macro_export]
macro_rules! message {
    ($id:literal $(, $name:ident = $value:expr)* $(,)?) => {{
        $(let $name = ($value).to_string();)*
        let english = i18n_embed_fl::fl!($crate::i18n::english().loader(), $id $(, $name = $name.clone())*);
        let args = std::collections::HashMap::from([$( (stringify!($name), $name), )*]);
        $crate::i18n::LocalizedMessage::new($id, args, english)
    }};
}

#[cfg(test)]
mod tests;
