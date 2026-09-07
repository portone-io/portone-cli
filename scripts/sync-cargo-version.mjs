// Synchronize the packages/@portone/cli/package.json version with
// Cargo.toml ([workspace.package]) and the portone-cli entry in Cargo.lock.
// This only replaces text, so it does not run Cargo, alter the rest of the
// lockfile, or produce different results when run repeatedly.

import * as fs from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const REPO_ROOT = resolve(fileURLToPath(import.meta.url), "../..");
const MANIFEST_PATH = resolve(REPO_ROOT, "packages/@portone/cli/package.json");

const { version } = JSON.parse(fs.readFileSync(MANIFEST_PATH, "utf-8"));
if (!version) {
  console.error(`No version found in ${MANIFEST_PATH}`);
  process.exit(1);
}

// Stop at the next section header so only [workspace.package].version changes.
syncFile(
  resolve(REPO_ROOT, "Cargo.toml"),
  /(\[workspace\.package\][^[]*?version = ")[^"]+(")/,
);
syncFile(
  resolve(REPO_ROOT, "Cargo.lock"),
  /(\[\[package\]\]\nname = "portone-cli"\nversion = ")[^"]+(")/,
);

function syncFile(path, pattern) {
  const content = fs.readFileSync(path, "utf-8");
  if (!pattern.test(content)) {
    console.error(`Version entry not found in ${path}`);
    process.exit(1);
  }
  const updated = content.replace(pattern, `$1${version}$2`);
  if (updated === content) {
    console.info(`Already up to date: ${path}`);
    return;
  }
  fs.writeFileSync(path, updated);
  console.info(`Update version to ${version} in ${path}`);
}
