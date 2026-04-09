# PortOne Codex Plugin

포트원(PortOne) 결제 연동 코드 생성 및 검토를 도와주는 Codex 전용 플러그인입니다.

## 기능

- 포트원 V1/V2 결제 연동 코드 생성
- 기존 연동 코드 검증 및 문제점 진단
- PortOne 공식 문서와 MCP 예시 기준 가이드 제공

## 필수 조건

이 플러그인을 사용하려면 `@portone/mcp-server` MCP 서버가 설정되어 있어야 합니다.

플러그인에 포함된 `.mcp.json` 기본 설정은 다음과 같습니다.

```json
{
  "mcpServers": {
    "portone": {
      "type": "stdio",
      "command": "npx",
      "args": ["-y", "@portone/mcp-server@latest"]
    }
  }
}
```

## 사용 방법

Codex에서 아래처럼 자연어로 요청하면 됩니다.

```text
포트원 V2 일반결제 연동 코드를 구현해줘
프로젝트의 포트원 연동 코드를 검토해줘
포트원 빌링키 결제 흐름을 추가해줘
```

## 포함된 스킬

- `payment-code-generator`: 신규 PortOne 연동 구현
- `integration-validator`: 기존 또는 생성된 PortOne 연동 검증
- `portone-guide`: PortOne 개념, 문서, MCP 활용 가이드

## 라이선스

MIT License
