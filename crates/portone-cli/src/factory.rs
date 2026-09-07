use std::cell::OnceCell;
use std::rc::Rc;
use std::sync::Arc;

use crate::auth::store::{KeyringStore, SecretStore};
use crate::config::Config;
use crate::i18n::Localizer;
use crate::ui::IoStreams;

pub struct Factory {
    pub localizer: Arc<Localizer>,
    pub io: IoStreams,
    config: OnceCell<Config>,
    agent: OnceCell<ureq::Agent>,
    store: OnceCell<Rc<dyn SecretStore>>,
}

impl Factory {
    pub fn detect() -> Self {
        Self::with_localizer(IoStreams::detect(), Localizer::detect())
    }

    pub fn new(io: IoStreams) -> Self {
        Self::with_localizer(io, Localizer::english())
    }

    pub fn with_localizer(io: IoStreams, localizer: Localizer) -> Self {
        Self {
            localizer: Arc::new(localizer),
            io,
            config: OnceCell::new(),
            agent: OnceCell::new(),
            store: OnceCell::new(),
        }
    }

    pub fn with_config(io: IoStreams, config: Config) -> Self {
        let factory = Self::new(io);
        let _ = factory.config.set(config);
        factory
    }

    pub fn with_store(io: IoStreams, config: Config, store: Rc<dyn SecretStore>) -> Self {
        let factory = Self::with_config(io, config);
        let _ = factory.store.set(store);
        factory
    }

    pub fn config(&self) -> anyhow::Result<&Config> {
        if let Some(config) = self.config.get() {
            return Ok(config);
        }
        let loaded = Config::load()?;
        Ok(self.config.get_or_init(|| loaded))
    }

    pub fn agent(&self) -> ureq::Agent {
        self.agent.get_or_init(crate::http::build_agent).clone()
    }

    pub fn secret_store(&self) -> Rc<dyn SecretStore> {
        self.store.get_or_init(|| Rc::new(KeyringStore)).clone()
    }
}
