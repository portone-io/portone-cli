# portone-cli

PortOne 결제/본인인증 연동을 위한 CLI 도구입니다. Claude Code와 Codex용 PortOne 플러그인을 각각 분리해 설치합니다.

## Usage

```bash
portone setup
portone setup --assistant claude
portone setup --assistant codex
portone setup --assistant both
```

## What It Sets Up

- Claude Code용 PortOne marketplace/plugin 설치
- Codex용 local plugin `plugins/portone-codex` 복사 및 `./.agents/plugins/marketplace.json` 구성
- Claude plugin `plugins/portone-integration` 와 Codex plugin `plugins/portone-codex` 를 별도 유지
