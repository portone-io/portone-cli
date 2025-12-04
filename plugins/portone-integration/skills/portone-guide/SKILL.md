---
name: PortOne Integration Guide
description: This skill should be used when the user asks about "포트원 연동", "PortOne integration", "결제 연동", "PG 연동", "billing key", "빌링키", "정기결제", "웹훅", "payment API", or needs guidance on implementing payment functionality with PortOne. Provides comprehensive guidance for V1 and V2 API integration.
version: 1.0.0
---

# PortOne Integration Guide

포트원(PortOne) 결제 연동에 필요한 핵심 개념과 MCP 도구 활용법을 안내한다.

## Overview

포트원은 온라인 결제, 본인인증, 파트너 정산 자동화를 위한 API와 SDK를 제공하는 서비스이다. 다양한 PG사(결제대행사)를 하나의 통합 API로 연동할 수 있어 개발 복잡도를 크게 줄일 수 있다.

## Version Selection: V1 vs V2

### V2 (권장)
- 신규 프로젝트에 권장
- 최신 SDK와 API 설계
- 더 나은 타입 안전성
- Bearer/PortOne 인증 스킴 지원
- **V2 API 호출 시 `PortOne` 인증 스킴 우선 사용**

### V1
- 레거시 프로젝트 지원
- 기존 연동 유지보수
- 일부 PG사 V1 전용 기능 존재

사용자가 버전을 지정하지 않은 경우, 신규 프로젝트면 V2를, 기존 코드가 있으면 해당 버전을 따른다.

## Payment Types

결제 유형을 선택할 때 다음 설명을 참고하여 사용자에게 안내한다:

### 일반결제 (인증결제)
- PG사 결제창을 통한 결제
- 카드, 계좌이체, 가상계좌, 휴대폰 결제 등
- 일회성 결제에 적합
- **사용 시점**: 쇼핑몰, 단건 상품 구매

### 정기결제 (빌링키 결제)
- 빌링키 발급 후 정기적으로 결제
- 구독 서비스, 월정액 서비스에 적합
- **사용 시점**: SaaS, 구독 서비스, 정기 배송

### 수기결제 (키인결제)
- 카드 정보 직접 입력 결제
- 고객 인증 없이 결제 가능
- B2B 거래, 전화 주문에 적합
- **사용 시점**: 콜센터, B2B 거래

### 본인인증
- 휴대폰/신용카드 본인인증
- 회원가입, 성인인증에 사용
- **사용 시점**: 회원가입, 연령 제한 서비스

## MCP Tools Usage

포트원 MCP 서버가 제공하는 도구들을 적극 활용한다.

### 코드 생성 시 필수 도구

**V2 코드 작성 전 반드시 예시 코드 조회:**

```
mcp__portone__readPortoneV2FrontendCode
- framework: react, html 중 선택
- pg: toss, nice, inicis 등
- pay_method: card, virtualAccount, easyPay 등

mcp__portone__readPortoneV2BackendCode
- framework: express, fastapi, flask, spring-kotlin 중 선택
- pg: 사용할 PG사
- pay_method: 결제 수단
```

### 문서 조회 도구

```
mcp__portone__listPortoneDocs
- 문서 목록 조회
- dev_docs, help_docs, release_notes 필터링

mcp__portone__readPortoneDoc
- 개별 문서 내용 조회
- path, fields 지정

mcp__portone__regexSearchPortoneDocs
- 정규식 기반 문서 검색
- 키워드로 관련 문서 찾기
```

### API 스키마 조회

```
mcp__portone__readPortoneOpenapiSchemaSummary
- V1/V2 API 전체 요약

mcp__portone__readPortoneOpenapiSchema
- 특정 API 상세 스키마
- yaml_path로 경로 지정
```

### 상점/채널 정보

```
mcp__portone__listStores
- 연결된 상점 정보

mcp__portone__getChannelsOfStore
- 상점의 채널 목록

mcp__portone__listSharedTestChannels
- 테스트용 공유 채널
```

## Integration Workflow

### 1. 환경 파악
- 프론트엔드/백엔드 프레임워크 확인
- 포트원 버전 결정 (V1/V2)
- 결제 유형 결정

### 2. 예시 코드 조회
- V2의 경우 `readPortoneV2FrontendCode`, `readPortoneV2BackendCode` 사용
- V1의 경우 `readPortoneDoc`으로 관련 문서 조회

### 3. 코드 생성
- 예시 코드를 프로젝트 구조에 맞게 수정
- 환경 변수로 API 키 관리
- 에러 핸들링 추가

### 4. 웹훅 연동
- 결제 상태 변경 알림 처리
- 가상계좌 입금 통보 등

## Code Generation Guidelines

### 프론트엔드

**SDK 로드 (V2):**
```javascript
import * as PortOne from "@portone/browser-sdk/v2";
```

**결제 요청:**
```javascript
const response = await PortOne.requestPayment({
  storeId: "store-id",
  channelKey: "channel-key",
  paymentId: `payment-${crypto.randomUUID()}`,
  orderName: "주문명",
  totalAmount: 1000,
  currency: "KRW",
  payMethod: "CARD",
});
```

### 백엔드

**결제 검증 (V2):**
```javascript
const response = await fetch(
  `https://api.portone.io/payments/${paymentId}`,
  {
    headers: {
      Authorization: `PortOne ${PORTONE_API_SECRET}`,
    },
  }
);
```

**중요**: V2 API 호출 시 `Bearer` 대신 `PortOne` 인증 스킴 우선 사용.

## Security Considerations

### API 키 관리
- API Secret은 절대 클라이언트에 노출하지 않음
- 환경 변수로 관리 (`.env` 파일)
- 커밋에서 제외 (`.gitignore`)

### 결제 검증
- 프론트엔드 결제 완료 후 반드시 서버에서 검증
- 금액 일치 여부 확인
- 결제 상태 확인

### 웹훅 검증
- 웹훅 시그니처 검증
- 중복 처리 방지 (멱등성)

## Error Handling

### 일반적인 에러 케이스
- 사용자 결제 취소
- 카드 한도 초과
- 네트워크 오류
- PG사 오류

### 에러 응답 처리
```javascript
if (response.code) {
  // 에러 발생
  console.error(response.message);
  // 사용자에게 적절한 메시지 표시
}
```

## Webhook Integration

### 웹훅 설정
- 포트원 콘솔에서 웹훅 URL 등록
- HTTPS 필수
- 타임아웃 고려

### 웹훅 처리
```javascript
app.post("/webhook", async (req, res) => {
  const { payment_id, status } = req.body;
  // 결제 상태 업데이트
  // 주문 처리
  res.status(200).send("OK");
});
```

## Common Integration Patterns

### 일반결제 플로우
1. 클라이언트: SDK로 결제창 호출
2. 사용자: 결제 진행
3. 클라이언트: 결제 결과 수신
4. 서버: 결제 검증 API 호출
5. 서버: 주문 상태 업데이트

### 정기결제 플로우
1. 클라이언트: 빌링키 발급 창 호출
2. 서버: 빌링키 저장
3. 서버: 정기 스케줄에 따라 결제 API 호출
4. 서버: 결제 결과 처리

## Testing

### 테스트 환경
- 포트원 테스트 채널 사용
- 실제 결제 없이 연동 테스트
- `mcp__portone__listSharedTestChannels`로 테스트 채널 조회

### 테스트 카드
- PG사별 테스트 카드 번호 사용
- 문서에서 테스트 정보 확인

## Additional Resources

### Reference Files
- **`references/v2-integration-details.md`** - V2 연동 상세 가이드
- **`references/webhook-patterns.md`** - 웹훅 처리 패턴

### MCP Documentation
포트원 MCP 서버 도구를 통해 최신 문서와 API 스키마를 항상 조회할 수 있다. 코드 작성 전 반드시 예시 코드와 문서를 확인한다.
