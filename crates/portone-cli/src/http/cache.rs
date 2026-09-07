use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

pub struct Cache {
    dir: PathBuf,
    ttl: Duration,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Envelope {
    status: u16,
    headers: Vec<(String, String)>,
}

impl Cache {
    pub fn new(ttl: Duration) -> Cache {
        Cache::with_dir(crate::config::paths::cache_dir().join("api"), ttl)
    }

    pub fn with_dir(dir: PathBuf, ttl: Duration) -> Cache {
        Cache { dir, ttl }
    }

    pub fn key(method: &str, url: &str, accept: &str, authorization: &str, body: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(format!("{method}:{url}:{accept}:{authorization}:"));
        hasher.update(body);
        let digest = hasher.finalize();

        use std::fmt::Write as _;
        let mut hex = String::with_capacity(digest.len() * 2);
        for byte in digest {
            let _ = write!(hex, "{byte:02x}");
        }
        hex
    }

    pub fn lookup(&self, key: &str) -> Option<CachedResponse> {
        let path = self.entry_path(key)?;
        let modified = fs::metadata(&path).ok()?.modified().ok()?;
        let fresh = match modified.checked_add(self.ttl) {
            Some(expires) => expires > SystemTime::now(),
            None => true,
        };
        if !fresh {
            return None;
        }

        let bytes = fs::read(&path).ok()?;
        let split = bytes.iter().position(|&b| b == b'\n')?;
        let envelope: Envelope = serde_json::from_slice(&bytes[..split]).ok()?;
        Some(CachedResponse {
            status: envelope.status,
            headers: envelope.headers,
            body: bytes[split + 1..].to_vec(),
        })
    }

    pub fn store(&self, key: &str, cacheable_method: bool, response: &CachedResponse) {
        if !cacheable_method {
            return;
        }
        if response.status >= 500 || response.status == 403 {
            return;
        }
        let Some(path) = self.entry_path(key) else {
            return;
        };
        let _ = write_entry(&path, response);
    }

    fn entry_path(&self, key: &str) -> Option<PathBuf> {
        let prefix = key.get(..2)?;
        let rest = key.get(2..)?;
        if rest.is_empty() {
            return None;
        }
        Some(self.dir.join(prefix).join(rest))
    }
}

fn write_entry(path: &Path, response: &CachedResponse) -> std::io::Result<()> {
    let parent = path.parent().ok_or(std::io::ErrorKind::InvalidInput)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt as _;
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o755)
            .create(parent)?;
    }
    #[cfg(not(unix))]
    fs::create_dir_all(parent)?;

    let envelope = Envelope {
        status: response.status,
        headers: response.headers.clone(),
    };
    let mut contents = serde_json::to_vec(&envelope)?;
    contents.push(b'\n');
    contents.extend_from_slice(&response.body);

    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    options.open(path)?.write_all(&contents)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_response(status: u16) -> CachedResponse {
        CachedResponse {
            status,
            headers: vec![
                ("content-type".into(), "application/json".into()),
                ("x-request-id".into(), "req-1".into()),
            ],
            body: b"line1\nline2\x00\xffend".to_vec(),
        }
    }

    fn entry_file(dir: &Path, key: &str) -> PathBuf {
        dir.join(&key[..2]).join(&key[2..])
    }

    #[test]
    fn key_is_stable_for_same_input() {
        let make = || {
            Cache::key(
                "GET",
                "https://api.portone.io/x",
                "application/json",
                "PortOne sk",
                b"body",
            )
        };
        let key = make();
        assert_eq!(key, make());
        assert_eq!(key.len(), 64);
        assert!(key.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn key_changes_when_any_input_changes() {
        let base = Cache::key("GET", "https://u", "a", "t", b"b");
        let variants = [
            Cache::key("HEAD", "https://u", "a", "t", b"b"),
            Cache::key("GET", "https://u2", "a", "t", b"b"),
            Cache::key("GET", "https://u", "a2", "t", b"b"),
            Cache::key("GET", "https://u", "a", "t2", b"b"),
            Cache::key("GET", "https://u", "a", "t", b"b2"),
        ];
        for variant in variants {
            assert_ne!(base, variant);
        }
    }

    #[test]
    fn store_then_lookup_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::with_dir(dir.path().to_path_buf(), Duration::from_secs(3600));
        let key = Cache::key("GET", "https://u", "a", "t", b"");
        let response = sample_response(200);

        cache.store(&key, true, &response);
        assert_eq!(cache.lookup(&key), Some(response));
    }

    #[test]
    fn zero_ttl_expires_immediately() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::with_dir(dir.path().to_path_buf(), Duration::ZERO);
        cache.store("cc33", true, &sample_response(200));
        assert!(entry_file(dir.path(), "cc33").exists());
        assert!(cache.lookup("cc33").is_none());
    }

    #[test]
    fn lookup_misses_after_mtime_expiry() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::with_dir(dir.path().to_path_buf(), Duration::from_secs(3600));
        cache.store("dd44", true, &sample_response(200));
        assert!(cache.lookup("dd44").is_some());

        let file = fs::File::options()
            .write(true)
            .open(entry_file(dir.path(), "dd44"))
            .unwrap();
        file.set_modified(SystemTime::now() - Duration::from_secs(7200))
            .unwrap();
        assert!(cache.lookup("dd44").is_none());
    }

    #[test]
    fn store_skips_uncacheable_method_403_and_5xx() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::with_dir(dir.path().to_path_buf(), Duration::from_secs(3600));

        cache.store("ee55", false, &sample_response(200));
        assert!(cache.lookup("ee55").is_none());
        cache.store("ff66", true, &sample_response(403));
        assert!(cache.lookup("ff66").is_none());
        cache.store("aa77", true, &sample_response(500));
        assert!(cache.lookup("aa77").is_none());

        cache.store("bb88", true, &sample_response(404));
        assert_eq!(cache.lookup("bb88").map(|r| r.status), Some(404));
    }

    #[test]
    fn store_overwrites_existing_entry() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::with_dir(dir.path().to_path_buf(), Duration::from_secs(3600));

        let mut first = sample_response(200);
        first.body = b"old".to_vec();
        cache.store("cc99", true, &first);
        let mut second = sample_response(200);
        second.body = b"new".to_vec();
        cache.store("cc99", true, &second);

        assert_eq!(cache.lookup("cc99").map(|r| r.body), Some(b"new".to_vec()));
    }

    #[test]
    fn lookup_ignores_malformed_entry() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::with_dir(dir.path().to_path_buf(), Duration::from_secs(3600));

        let path = entry_file(dir.path(), "dd00");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"not-json\nbody").unwrap();
        assert!(cache.lookup("dd00").is_none());
        fs::write(&path, b"{\"status\":200,\"headers\":[]}").unwrap();
        assert!(cache.lookup("dd00").is_none());
    }

    #[cfg(unix)]
    #[test]
    fn stored_entry_has_restrictive_modes() {
        use std::os::unix::fs::PermissionsExt as _;

        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::with_dir(dir.path().to_path_buf(), Duration::from_secs(3600));
        cache.store("ee11", true, &sample_response(200));

        let file = entry_file(dir.path(), "ee11");
        let file_mode = fs::metadata(&file).unwrap().permissions().mode() & 0o777;
        assert_eq!(file_mode, 0o600);
        let dir_mode = fs::metadata(file.parent().unwrap())
            .unwrap()
            .permissions()
            .mode()
            & 0o700;
        assert_eq!(dir_mode, 0o700);
    }
}
