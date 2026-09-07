use std::path::PathBuf;

use etcetera::BaseStrategy;

fn env_override(name: &str) -> Option<PathBuf> {
    let value = std::env::var(name).ok()?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(PathBuf::from(trimmed))
    }
}

fn base_strategy() -> impl BaseStrategy {
    etcetera::choose_base_strategy().expect("unable to determine the home directory")
}

pub fn config_dir() -> PathBuf {
    env_override("PORTONE_CONFIG_DIR")
        .unwrap_or_else(|| base_strategy().config_dir().join("portone"))
}

pub(crate) fn try_config_dir() -> Option<PathBuf> {
    env_override("PORTONE_CONFIG_DIR").or_else(|| {
        etcetera::choose_base_strategy()
            .ok()
            .map(|base| base.config_dir().join("portone"))
    })
}

pub fn cache_dir() -> PathBuf {
    env_override("PORTONE_CACHE_DIR").unwrap_or_else(|| base_strategy().cache_dir().join("portone"))
}

#[cfg(test)]
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
pub(crate) fn with_env(vars: &[(&str, Option<&str>)], f: impl FnOnce()) {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|err| err.into_inner());
    let saved: Vec<(String, Option<String>)> = vars
        .iter()
        .map(|(key, _)| ((*key).to_string(), std::env::var(key).ok()))
        .collect();
    for (key, value) in vars {
        unsafe {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    for (key, value) in saved {
        unsafe {
            match value {
                Some(value) => std::env::set_var(&key, value),
                None => std::env::remove_var(&key),
            }
        }
    }
    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_dir_env_override() {
        with_env(&[("PORTONE_CONFIG_DIR", Some("/custom/config"))], || {
            assert_eq!(config_dir(), PathBuf::from("/custom/config"));
        });
    }

    #[test]
    fn cache_dir_env_override() {
        with_env(&[("PORTONE_CACHE_DIR", Some("/custom/cache"))], || {
            assert_eq!(cache_dir(), PathBuf::from("/custom/cache"));
        });
    }

    #[test]
    fn empty_override_falls_back_to_default() {
        with_env(
            &[
                ("PORTONE_CONFIG_DIR", Some("   ")),
                ("PORTONE_CACHE_DIR", None),
            ],
            || {
                assert!(config_dir().ends_with("portone"));
                assert!(cache_dir().ends_with("portone"));
            },
        );
    }
}
