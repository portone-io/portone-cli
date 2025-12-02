---
description: 기존 포트원 연동 코드를 검토하고 개선점을 제안합니다
argument-hint: [파일경로 또는 디렉토리]
allowed-tools: Read, Glob, Grep, mcp__portone__listPortoneDocs, mcp__portone__readPortoneDoc, mcp__portone__regex_search_portone_docs, mcp__portone__readPortoneOpenapiSchema, mcp__portone__readPortoneOpenapiSchemaSummary
---

# 포트원 연동 코드 검토

기존 프로젝트의 포트원 결제 연동 코드를 분석하고 개선점을 제안한다.

## 검토 대상 파악

$ARGUMENTS가 제공된 경우 해당 파일/디렉토리를 검토.
제공되지 않은 경우 프로젝트 전체에서 포트원 관련 코드를 탐색.

## 워크플로우

### Step 1: 포트원 관련 코드 탐색

프로젝트에서 포트원 관련 코드 검색:

```
Grep 패턴:
- "portone" (대소문자 무관)
- "@portone/browser-sdk"
- "IMP\." 또는 "Iamport"
- "api.portone.io"
- "api.iamport.kr"
- "imp_uid"
- "merchant_uid"
- "paymentId"
- "billingKey"
```

### Step 2: 버전 감지

코드 패턴으로 V1/V2 판별:

**V2 패턴:**
- `@portone/browser-sdk`
- `PortOne.requestPayment`
- `api.portone.io`
- `storeId`, `channelKey`

**V1 패턴:**
- `iamport-react-native` 또는 유사 라이브러리
- `IMP.request_pay`
- `api.iamport.kr`
- `imp_uid`, `merchant_uid`

### Step 3: 코드 분석

발견된 파일들을 읽고 분석:

**프론트엔드 코드 체크포인트:**
- SDK 초기화 방식
- 결제 요청 파라미터
- 결제 결과 처리 로직
- 에러 핸들링

**백엔드 코드 체크포인트:**
- 결제 검증 API
- 인증 방식 (API 키 사용)
- 웹훅 처리
- 에러 핸들링

### Step 4: 베스트 프랙티스 조회

MCP 도구로 최신 권장사항 확인:

```
mcp__portone__listPortoneDocs
- 관련 문서 목록 조회

mcp__portone__readPortoneDoc
- 해당 버전/결제유형 문서 내용 확인

mcp__portone__regex_search_portone_docs
- 특정 기능/이슈 관련 문서 검색
```

### Step 5: 검토 보고서 작성

다음 항목들을 체크하고 보고:

## 검토 항목

### 1. 보안 (Critical)

- [ ] **API Secret 노출 여부**
  - 클라이언트 코드에 API Secret이 포함되어 있지 않은지
  - 환경 변수로 관리되고 있는지
  - `.gitignore`에 `.env` 파일이 포함되어 있는지

- [ ] **결제 검증**
  - 프론트엔드 결제 완료 후 서버에서 검증하는지
  - 금액 일치 여부 확인 로직이 있는지
  - 결제 상태 확인 로직이 있는지

- [ ] **웹훅 검증**
  - 웹훅 시그니처 검증이 있는지 (V2)
  - IP 화이트리스트 설정 여부

### 2. 에러 핸들링 (High)

- [ ] **사용자 취소 처리**
  - 결제 취소 시 적절한 UX 제공

- [ ] **결제 실패 처리**
  - 실패 사유별 메시지 표시
  - 재시도 로직 여부

- [ ] **네트워크 오류 처리**
  - 타임아웃 처리
  - 재시도 로직

### 3. 코드 품질 (Medium)

- [ ] **SDK 버전**
  - 최신 버전 사용 여부
  - 더 이상 지원되지 않는 API 사용 여부

- [ ] **타입 안전성**
  - TypeScript 사용 시 타입 정의
  - 결제 응답 타입 처리

- [ ] **코드 구조**
  - 결제 로직 분리 여부
  - 재사용 가능한 구조

### 4. 기능 완성도 (Medium)

- [ ] **웹훅 연동**
  - 웹훅 엔드포인트 구현 여부
  - 멱등성 처리

- [ ] **결제 취소 기능**
  - 환불 API 연동 여부

- [ ] **결제 내역 조회**
  - 결제 상태 조회 기능

### 5. V1→V2 마이그레이션 (해당 시)

V1 코드가 발견된 경우:
- V2 마이그레이션 권장 여부 평가
- 마이그레이션 시 변경 필요 사항 안내

## 보고서 형식

```markdown
# 포트원 연동 코드 검토 보고서

## 개요
- 감지된 버전: V1/V2
- 결제 유형: 일반결제/정기결제/...
- 검토 파일 수: N개

## 발견 사항

### Critical (즉시 수정 필요)
1. [파일:라인] 이슈 설명
   - 현재 코드
   - 권장 수정

### High (권장 수정)
...

### Medium (개선 권장)
...

## 권장 사항
1. ...
2. ...

## 참고 문서
- [포트원 문서 링크]
```

## 사용 예시

```
/review-integration
/review-integration src/payment/
/review-integration src/api/payment.ts
```
