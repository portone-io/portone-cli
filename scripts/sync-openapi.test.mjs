import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import {
  chmodSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

const syncScript = fileURLToPath(new URL("./sync-openapi.sh", import.meta.url));
const schemaPath = "crates/portone-schema-macros/schema/v2.openapi.json";
const checks = [
  "run --locked --package xtask -- gen-docs",
  "fmt --all --check",
  "clippy --locked --workspace --all-targets -- -D warnings",
  "test --locked --workspace",
  "run --locked --package xtask -- gen-docs --check",
  "vet --locked",
];

function git(cwd, ...args) {
  return execFileSync("git", args, {
    cwd,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
    env: {
      ...process.env,
      GIT_CONFIG_GLOBAL: "/dev/null",
      GIT_CONFIG_NOSYSTEM: "1",
    },
  }).trim();
}

test("schema sync validates before publishing and reuses one PR without duplicate commits", () => {
  const root = mkdtempSync(join(tmpdir(), "portone-schema-sync-"));
  try {
    const remote = join(root, "remote.git");
    const seed = join(root, "seed");
    const bin = join(root, "bin");
    const events = join(root, "events.jsonl");
    const checkLog = join(root, "checks.log");
    const source = join(root, "source.json");
    const state = join(root, "pr-state");
    mkdirSync(bin);
    git(root, "init", "--bare", "--initial-branch=main", remote);
    git(root, "clone", remote, seed);
    mkdirSync(join(seed, "crates/portone-schema-macros/schema"), {
      recursive: true,
    });
    mkdirSync(join(seed, "docs/reference"), { recursive: true });
    writeFileSync(join(seed, schemaPath), '{"version":1}\n');
    writeFileSync(join(seed, "docs/reference/index.md"), "Reference\n");
    git(seed, "add", ".");
    git(
      seed,
      "-c",
      "user.name=Test",
      "-c",
      "user.email=test@example.invalid",
      "commit",
      "-m",
      "Initial",
    );
    git(seed, "push", "origin", "main");

    writeFileSync(
      join(bin, "cargo"),
      `#!/usr/bin/env bash
set -eu
printf '%s\\n' "$*" >> "$CHECK_LOG"
if [[ "$*" == "$FAKE_CARGO_FAIL" ]]; then exit 1; fi
`,
    );
    writeFileSync(
      join(bin, "gh"),
      `#!/usr/bin/env node
const fs = require('node:fs');
const args = process.argv.slice(2);
const bodyIndex = args.indexOf('--body-file');
const event = {args, body: bodyIndex < 0 ? null : fs.readFileSync(args[bodyIndex + 1], 'utf8')};
fs.appendFileSync(process.env.EVENTS, JSON.stringify(event) + '\\n');
if (args[0] !== 'pr') process.exit(2);
if (args[1] === 'list' && fs.existsSync(process.env.PR_STATE)) process.stdout.write('1\\n');
else if (args[1] === 'create') fs.writeFileSync(process.env.PR_STATE, '1');
else if (!['list', 'edit'].includes(args[1])) process.exit(2);
`,
    );
    chmodSync(join(bin, "cargo"), 0o755);
    chmodSync(join(bin, "gh"), 0o755);
    let runNumber = 0;
    function run(contents, failure = "") {
      const checkout = join(root, `checkout-${runNumber++}`);
      git(root, "clone", remote, checkout);
      writeFileSync(source, contents);
      writeFileSync(checkLog, "");
      return spawnSync("bash", [syncScript, source, "a".repeat(40)], {
        cwd: checkout,
        encoding: "utf8",
        env: {
          ...process.env,
          GIT_CONFIG_GLOBAL: "/dev/null",
          GIT_CONFIG_NOSYSTEM: "1",
          PATH: `${bin}:${process.env.PATH}`,
          CHECK_LOG: checkLog,
          EVENTS: events,
          PR_STATE: state,
          FAKE_CARGO_FAIL: failure,
        },
      });
    }
    function succeeded(output) {
      assert.equal(output.status, 0, output.stdout + output.stderr);
      assert.deepEqual(
        readFileSync(checkLog, "utf8").trim().split("\n"),
        checks,
      );
    }
    function log() {
      try {
        return readFileSync(events, "utf8")
          .trim()
          .split("\n")
          .filter(Boolean)
          .map(JSON.parse);
      } catch (error) {
        if (error.code === "ENOENT") return [];
        throw error;
      }
    }

    succeeded(run('{"version":1}\n'));
    assert.deepEqual(log(), []);
    assert.equal(
      git(seed, "ls-remote", "--heads", "origin", "chore/openapi"),
      "",
    );

    succeeded(run('{"version":2}\n'));
    const first = git(remote, "rev-parse", "chore/openapi");
    assert.equal(
      git(remote, "show", `chore/openapi:${schemaPath}`),
      '{"version":2}',
    );
    assert.deepEqual(
      log().map((event) => event.args[1]),
      ["list", "create"],
    );
    assert.match(log().at(-1).body, /commit\/a{40}/);
    assert.match(log().at(-1).body, /\n\n검증:/);

    succeeded(run('{"version":2}\n'));
    assert.equal(git(remote, "rev-parse", "chore/openapi"), first);
    assert.equal(log().at(-1).args[1], "edit");

    succeeded(run('{"version":3}\n'));
    const updated = git(remote, "rev-parse", "chore/openapi");
    assert.notEqual(updated, first);
    assert.equal(log().filter((event) => event.args[1] === "create").length, 1);
    const eventsBeforeFailure = log();
    for (const failure of checks) {
      const output = run('{"version":4}\n', failure);
      assert.notEqual(output.status, 0, `must stop at ${failure}`);
      assert.equal(git(remote, "rev-parse", "chore/openapi"), updated);
      assert.deepEqual(log(), eventsBeforeFailure);
    }
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
