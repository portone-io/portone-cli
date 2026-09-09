use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const SOURCE: &str = "skills/portone-cli";
const DESTINATIONS: [&str; 2] = [
    "plugins/portone-codex/skills/portone-cli",
    "plugins/portone-integration/skills/portone-cli",
];

#[derive(Debug, Eq, PartialEq)]
enum Entry {
    Directory,
    File(Vec<u8>),
}

impl Entry {
    fn same_kind(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::Directory, Self::Directory) | (Self::File(_), Self::File(_))
        )
    }
}

type Tree = BTreeMap<PathBuf, Entry>;

#[derive(Debug, Eq, PartialEq)]
pub struct Difference {
    kind: &'static str,
    path: PathBuf,
}

impl fmt::Display for Difference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind, self.path.display())
    }
}

/// Check every tree before writing, so unsupported entries cannot produce a
/// partially synchronized bundle. Only the generated skill roots are managed.
pub fn run(workspace: &Path, check: bool) -> io::Result<Vec<Difference>> {
    let source = workspace.join(SOURCE);
    let expected = read_tree(&source)?;
    if expected.get(Path::new("")) != Some(&Entry::Directory) {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("canonical skill directory not found: {}", source.display()),
        ));
    }
    let destinations = DESTINATIONS
        .iter()
        .map(|destination| {
            let path = workspace.join(destination);
            let actual = read_tree(&path)?;
            Ok((path, actual))
        })
        .collect::<io::Result<Vec<_>>>()?;

    let mut differences = Vec::new();
    for (path, actual) in &destinations {
        for (relative, entry) in &expected {
            let kind = match actual.get(relative) {
                None => "missing",
                Some(existing) if existing != entry => "different",
                Some(_) => continue,
            };
            differences.push(Difference {
                kind,
                path: entry_path(path, relative),
            });
        }
        for relative in actual.keys().filter(|key| !expected.contains_key(*key)) {
            differences.push(Difference {
                kind: "stale",
                path: entry_path(path, relative),
            });
        }
    }
    differences.sort_by(|left, right| left.path.cmp(&right.path));

    if !check {
        for (path, actual) in &destinations {
            write_tree(path, &expected, actual)?;
        }
    }
    Ok(differences)
}

fn read_tree(root: &Path) -> io::Result<Tree> {
    let mut tree = Tree::new();
    match fs::symlink_metadata(root) {
        Ok(_) => read_entry(root, Path::new(""), &mut tree)?,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {}
        Err(err) => return Err(err),
    }
    Ok(tree)
}

fn read_entry(root: &Path, relative: &Path, tree: &mut Tree) -> io::Result<()> {
    let path = entry_path(root, relative);
    let kind = fs::symlink_metadata(&path)?.file_type();
    if kind.is_file() {
        tree.insert(relative.to_path_buf(), Entry::File(fs::read(&path)?));
    } else if kind.is_dir() {
        tree.insert(relative.to_path_buf(), Entry::Directory);
        for child in fs::read_dir(path)? {
            read_entry(root, &relative.join(child?.file_name()), tree)?;
        }
    } else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "only regular files and directories are supported (no symlinks): {}",
                path.display()
            ),
        ));
    }
    Ok(())
}

fn entry_path(root: &Path, relative: &Path) -> PathBuf {
    // Joining an empty path adds a trailing separator, which follows a root
    // symlink and fails for a root file on Unix.
    if relative.as_os_str().is_empty() {
        root.to_path_buf()
    } else {
        root.join(relative)
    }
}

fn write_tree(root: &Path, expected: &Tree, actual: &Tree) -> io::Result<()> {
    // Descendants precede parents so obsolete directories are empty when removed.
    for (relative, entry) in actual.iter().rev() {
        if expected
            .get(relative)
            .is_some_and(|desired| desired.same_kind(entry))
        {
            continue;
        }
        let path = entry_path(root, relative);
        match entry {
            Entry::Directory => fs::remove_dir(path)?,
            Entry::File(_) => fs::remove_file(path)?,
        }
    }
    for (relative, entry) in expected {
        if actual.get(relative) == Some(entry) {
            continue;
        }
        let path = entry_path(root, relative);
        match entry {
            Entry::Directory => fs::create_dir_all(path)?,
            Entry::File(content) => fs::write(path, content)?,
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct Workspace(PathBuf);

    impl Workspace {
        fn new() -> Self {
            static NEXT_ID: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "portone-sync-skills-{}-{}",
                std::process::id(),
                NEXT_ID.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            fs::create_dir_all(path.join(SOURCE)).unwrap();
            Self(path)
        }

        fn write(&self, relative: impl AsRef<Path>, content: impl AsRef<[u8]>) {
            let path = self.0.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, content).unwrap();
        }
    }

    impl Drop for Workspace {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn copies_nested_regular_files_to_both_bundles_and_is_idempotent() {
        let workspace = Workspace::new();
        workspace.write(format!("{SOURCE}/SKILL.md"), "canonical\n");
        workspace.write(format!("{SOURCE}/references/schema.bin"), [0, 128, 255]);
        fs::create_dir(workspace.0.join(SOURCE).join("empty")).unwrap();
        for destination in DESTINATIONS {
            let sibling = Path::new(destination)
                .parent()
                .unwrap()
                .join("other/SKILL.md");
            workspace.write(sibling, "preserve me");
        }

        assert!(!run(&workspace.0, false).unwrap().is_empty());
        for destination in DESTINATIONS {
            assert_eq!(
                read_tree(&workspace.0.join(destination)).unwrap(),
                read_tree(&workspace.0.join(SOURCE)).unwrap()
            );
            let sibling = Path::new(destination)
                .parent()
                .unwrap()
                .join("other/SKILL.md");
            assert_eq!(fs::read(workspace.0.join(sibling)).unwrap(), b"preserve me");
        }
        assert!(run(&workspace.0, false).unwrap().is_empty());
        assert!(run(&workspace.0, true).unwrap().is_empty());
    }

    #[test]
    fn check_reports_missing_different_and_stale_without_writing() {
        let workspace = Workspace::new();
        workspace.write(format!("{SOURCE}/SKILL.md"), "canonical");
        workspace.write(format!("{SOURCE}/references/new.md"), "new");
        workspace.write(format!("{}/SKILL.md", DESTINATIONS[0]), "old");
        workspace.write(format!("{}/obsolete/old.md", DESTINATIONS[0]), "stale");
        let before = read_tree(&workspace.0).unwrap();

        let differences = run(&workspace.0, true).unwrap();

        assert!(differences.contains(&Difference {
            kind: "different",
            path: workspace.0.join(DESTINATIONS[0]).join("SKILL.md"),
        }));
        assert!(differences.contains(&Difference {
            kind: "missing",
            path: workspace.0.join(DESTINATIONS[0]).join("references/new.md"),
        }));
        assert!(differences.contains(&Difference {
            kind: "stale",
            path: workspace.0.join(DESTINATIONS[0]).join("obsolete/old.md"),
        }));
        assert!(!workspace.0.join(DESTINATIONS[1]).exists());
        assert_eq!(before, read_tree(&workspace.0).unwrap());

        run(&workspace.0, false).unwrap();
        assert!(!workspace.0.join(DESTINATIONS[0]).join("obsolete").exists());
        assert!(run(&workspace.0, true).unwrap().is_empty());
    }

    #[test]
    fn replaces_file_directory_conflicts_and_stale_nested_directories() {
        let workspace = Workspace::new();
        workspace.write(format!("{SOURCE}/references/new.md"), "new");
        workspace.write(format!("{SOURCE}/SKILL.md"), "canonical");
        workspace.write(format!("{}/references", DESTINATIONS[0]), "old file");
        workspace.write(format!("{}/SKILL.md/old/file.md", DESTINATIONS[0]), "old");
        workspace.write(DESTINATIONS[1], "old root file");

        run(&workspace.0, false).unwrap();

        assert!(run(&workspace.0, true).unwrap().is_empty());
        assert_eq!(
            fs::read(workspace.0.join(DESTINATIONS[0]).join("SKILL.md")).unwrap(),
            b"canonical"
        );
    }

    #[test]
    fn missing_source_does_not_delete_generated_skills() {
        let workspace = Workspace::new();
        workspace.write(format!("{}/SKILL.md", DESTINATIONS[0]), "keep");
        fs::remove_dir(workspace.0.join(SOURCE)).unwrap();
        let before = read_tree(&workspace.0).unwrap();

        assert!(run(&workspace.0, false).is_err());

        assert_eq!(before, read_tree(&workspace.0).unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn refuses_source_and_destination_symlinks_before_writing() {
        use std::os::unix::fs::symlink;

        let workspace = Workspace::new();
        workspace.write(format!("{SOURCE}/SKILL.md"), "canonical");
        workspace.write("outside.md", "outside");
        let source_link = workspace.0.join(SOURCE).join("link.md");
        symlink(workspace.0.join("outside.md"), &source_link).unwrap();
        assert!(run(&workspace.0, false).is_err());
        assert!(!workspace.0.join(DESTINATIONS[0]).exists());
        fs::remove_file(source_link).unwrap();

        let destination_link = workspace.0.join(DESTINATIONS[1]);
        fs::create_dir_all(destination_link.parent().unwrap()).unwrap();
        symlink(workspace.0.join(SOURCE), &destination_link).unwrap();
        assert!(run(&workspace.0, true).is_err());
        assert!(run(&workspace.0, false).is_err());
        assert!(!workspace.0.join(DESTINATIONS[0]).exists());
        assert_eq!(
            fs::read(workspace.0.join("outside.md")).unwrap(),
            b"outside"
        );
    }
}
