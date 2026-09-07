use std::path::Path;

use crate::i18n::LocalizedContext;
use include_dir::{Dir, include_dir};

static PORTONE_CODEX_PLUGIN: Dir<'static> =
    include_dir!("$CARGO_MANIFEST_DIR/../../plugins/portone-codex");

pub fn extract(target_dir: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(target_dir).with_lcontext(|| {
        crate::message!("setup-create-directory-failed", path = target_dir.display())
    })?;
    PORTONE_CODEX_PLUGIN.extract(target_dir).with_lcontext(|| {
        crate::message!("setup-extract-assets-failed", path = target_dir.display())
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_writes_plugin_files() {
        let dir = tempfile::tempdir().unwrap();
        extract(dir.path()).unwrap();

        assert!(dir.path().join(".codex-plugin/plugin.json").is_file());
        assert!(dir.path().join(".mcp.json").is_file());
    }

    #[test]
    fn extract_overwrites_existing_files() {
        let dir = tempfile::tempdir().unwrap();
        let plugin_json = dir.path().join(".codex-plugin/plugin.json");
        std::fs::create_dir_all(plugin_json.parent().unwrap()).unwrap();
        std::fs::write(&plugin_json, "stale").unwrap();

        extract(dir.path()).unwrap();

        let content = std::fs::read_to_string(&plugin_json).unwrap();
        assert_ne!(content, "stale");
    }
}
