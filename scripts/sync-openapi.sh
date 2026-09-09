#!/usr/bin/env bash
# Run by developers.portone.io after checking out a clean portone-cli main.
# Git authentication is provided by `gh auth setup-git` and GH_TOKEN.
set -euo pipefail

schema_source=${1:?usage: sync-openapi.sh SOURCE_JSON SOURCE_COMMIT}
source_commit=${2:?usage: sync-openapi.sh SOURCE_JSON SOURCE_COMMIT}
cli_repo=${CLI_REPO:-portone-io/portone-cli}
sync_branch=chore/openapi
schema_target=crates/portone-schema-macros/schema/v2.openapi.json

if [[ ! "$source_commit" =~ ^[0-9a-f]{40}$ ]]; then
  echo "Expected a full source commit SHA" >&2
  exit 1
fi
if [[ ! -f "$schema_target" ]]; then
  echo "The schema macro must be merged into CLI main before enabling sync" >&2
  exit 1
fi
if ! git diff --quiet || ! git diff --cached --quiet; then
  echo "Schema sync requires a clean checkout" >&2
  exit 1
fi

cp "$schema_source" "$schema_target"

# Complete every check before touching the remote branch or pull request.
cargo run --locked --package xtask -- gen-docs
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo run --locked --package xtask -- gen-docs --check
cargo vet --locked

git add -- "$schema_target" docs/reference
if git diff --cached --quiet; then
  echo "OpenAPI schema and documentation already match main"
  exit 0
fi

remote_ref=$(git ls-remote --heads origin "refs/heads/$sync_branch")
previous_commit=
if [[ -n "$remote_ref" ]]; then
  git fetch origin "refs/heads/$sync_branch:refs/remotes/origin/$sync_branch"
  previous_commit=$(git rev-parse "refs/remotes/origin/$sync_branch")
fi

candidate_tree=$(git write-tree)
previous_tree=
if [[ -n "$previous_commit" ]]; then
  previous_tree=$(git rev-parse "$previous_commit^{tree}")
fi
if [[ "$candidate_tree" != "$previous_tree" ]]; then
  git switch -C "$sync_branch"
  git -c user.name='github-actions[bot]' \
    -c user.email='github-actions[bot]@users.noreply.github.com' \
    commit -m "chore: update OpenAPI schema"
  git push --force-with-lease="refs/heads/$sync_branch:$previous_commit" \
    origin "HEAD:refs/heads/$sync_branch"
else
  echo "Existing schema sync branch already has these contents"
fi

pr_number=$(gh pr list --repo "$cli_repo" --head "$sync_branch" --base main \
  --state open --json number --jq '.[0].number')
body_file=$(mktemp)
trap 'rm -f "$body_file"' EXIT
cat > "$body_file" <<EOF
개발자센터의 원본 OpenAPI 스키마와 CLI 참조 문서를 동기화합니다.

원본: [developers.portone.io@$source_commit](https://github.com/portone-io/developers.portone.io/commit/$source_commit)

검증: workspace 형식 검사, Clippy, 테스트, 참조 문서 최신성 검사, cargo vet 통과.
EOF
if [[ -n "$pr_number" ]]; then
  gh pr edit "$pr_number" --repo "$cli_repo" \
    --title "chore: update OpenAPI schema" --body-file "$body_file"
else
  gh pr create --repo "$cli_repo" --base main --head "$sync_branch" \
    --title "chore: update OpenAPI schema" --body-file "$body_file"
fi
