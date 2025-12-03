# PortOne Integration Plugin

포트원(PortOne) 결제 연동 코드 생성 및 검토를 도와주는 Claude Code 플러그인입니다.

## 기능

### 코드 생성
- 포트원 V1/V2 결제 연동 코드 자동 생성
- 프론트엔드/백엔드 프레임워크별 맞춤 코드
- 일반결제, 정기결제, 수기결제, 본인인증 지원

### 코드 검토
- 기존 연동 코드 보안 점검
- 베스트 프랙티스 준수 여부 확인
- 개선 권장사항 제공

## 필수 조건

이 플러그인을 사용하려면 `@portone/mcp-server` MCP 서버가 설정되어 있어야 합니다.

### MCP 서버 설정

프로젝트의 `.mcp.json` 파일에 다음 내용을 추가하세요:

```json
{
  "mcpServers": {
    "portone": {
      "type": "stdio",
      "command": "npx",
      "args": ["@portone/mcp-server@latest"]
    }
  }
}
```

## 사용 방법

### 슬래시 커맨드

#### `/start`

결제 연동 코드를 대화형으로 생성합니다.

```
/portone-integration:start                    # 대화형으로 모든 옵션 선택
/portone-integration:start v2                 # V2 버전으로 생성
/portone-integration:start v2 checkout        # V2 일반결제 코드 생성
/portone-integration:start v1 billing         # V1 정기결제 코드 생성
```

**결제 유형:**
- `checkout`: 일반결제 (PG 결제창)
- `billing`: 정기결제 (빌링키 방식)
- `keyin`: 수기결제 (카드 직접 입력)
- `identity`: 본인인증

#### `/review`

기존 포트원 연동 코드를 검토합니다.

```
/portone-integration:review                  # 프로젝트 전체 검토
/portone-integration:review src/payment/     # 특정 디렉토리 검토
/portone-integration:review src/api/pay.ts   # 특정 파일 검토
```

### 에이전트

플러그인은 다음 상황에서 자동으로 에이전트를 활성화합니다:

#### Payment Code Generator
- "포트원 결제 기능 구현해줘"
- "정기결제 연동 코드 작성해줘"
- "카드 결제 코드 만들어줘"

#### Integration Reviewer
- "포트원 연동 코드 검토해줘"
- "결제 연동 보안 점검해줘"
- "PG 연동 코드 리뷰해줘"

### 스킬

"포트원 연동", "결제 연동", "빌링키" 등의 키워드로 질문하면 관련 가이드가 자동으로 로드됩니다.

## 지원 프레임워크

### 프론트엔드
- React
- HTML (Vanilla JS)
- Vue (HTML 기반 적용)

### 백엔드
- Express (Node.js)
- FastAPI (Python)
- Flask (Python)
- Spring/Kotlin

## 결제 유형 안내

### 일반결제 (Checkout)
PG사 결제창을 통한 인증 결제입니다. 카드, 계좌이체, 가상계좌, 휴대폰 결제를 지원합니다.
- 적합한 서비스: 쇼핑몰, 단건 상품 구매

### 정기결제 (Billing)
빌링키를 발급받아 정기적으로 결제합니다.
- 적합한 서비스: SaaS, 구독 서비스, 멤버십

### 수기결제 (Keyin)
카드 정보를 직접 입력하여 결제합니다. V2에서만 완전 지원됩니다.
- 적합한 서비스: 콜센터, B2B 거래

### 본인인증 (Identity)
휴대폰 또는 신용카드로 본인인증을 수행합니다.
- 적합한 서비스: 회원가입, 성인인증

## V1 vs V2

### V2 (권장)
- 신규 프로젝트에 권장
- 최신 SDK 설계
- 더 나은 타입 안전성
- `PortOne` 인증 스킴 지원

### V1
- 레거시 프로젝트 지원
- 기존 연동 유지보수

## 보안 주의사항

- API Secret은 절대 클라이언트 코드에 노출하지 마세요
- 결제 완료 후 반드시 서버에서 검증하세요
- 환경 변수로 모든 인증 정보를 관리하세요
- `.env` 파일을 `.gitignore`에 추가하세요

## 라이선스

MIT License
