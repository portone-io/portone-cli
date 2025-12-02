---
description: 포트원 결제 연동 코드를 대화형으로 생성합니다
argument-hint: [옵션: v1|v2] [결제유형: checkout|billing|keyin|identity]
allowed-tools: Read, Write, Glob, Grep, AskUserQuestion, mcp__portone__readPortoneV2FrontendCode, mcp__portone__readPortoneV2BackendCode, mcp__portone__readPortoneOpenapiSchema, mcp__portone__readPortoneOpenapiSchemaSummary, mcp__portone__listPortoneDocs, mcp__portone__readPortoneDoc, mcp__portone__regex_search_portone_docs, mcp__portone__listStores, mcp__portone__getChannelsOfStore, mcp__portone__listSharedTestChannels
---

# 포트원 결제 연동 코드 생성

사용자의 프로젝트에 맞는 포트원 결제 연동 코드를 생성한다.

## 입력 분석

인자가 제공된 경우:
- $1: 포트원 버전 (v1 또는 v2)
- $2: 결제 유형 (checkout, billing, keyin, identity)

## 워크플로우

### Step 1: 프로젝트 환경 파악

먼저 현재 프로젝트의 기술 스택을 파악한다:
- package.json, requirements.txt, build.gradle 등 확인
- 프론트엔드 프레임워크 (React, Vue, HTML 등)
- 백엔드 프레임워크 (Express, FastAPI, Spring 등)

### Step 2: 포트원 버전 결정

$1이 지정되지 않은 경우 AskUserQuestion으로 확인:

**V2 (권장 - 신규 프로젝트)**
- 최신 SDK 설계
- 더 나은 타입 안전성
- PortOne 인증 스킴

**V1 (레거시 프로젝트)**
- 기존 연동 유지보수
- 일부 PG사 V1 전용 기능

### Step 3: 결제 유형 선택

$2가 지정되지 않은 경우 AskUserQuestion으로 확인:

**일반결제 (checkout)**
- PG사 결제창을 통한 인증 결제
- 카드, 계좌이체, 가상계좌, 휴대폰 결제
- 쇼핑몰, 단건 상품 구매에 적합

**정기결제 (billing)**
- 빌링키 발급 후 정기 결제
- 구독 서비스, 월정액에 적합
- SaaS, 멤버십 서비스

**수기결제 (keyin)**
- 카드 정보 직접 입력 결제
- 전화 주문, B2B 거래에 적합
- V2에서만 완전 지원

**본인인증 (identity)**
- 휴대폰/신용카드 본인인증
- 회원가입, 성인인증에 사용

### Step 4: 구현 범위 결정

AskUserQuestion으로 확인:
- 프론트엔드만
- 백엔드만
- 프론트엔드 + 백엔드 모두

### Step 5: 프레임워크 선택

사용자의 프로젝트 환경에 맞게:

**프론트엔드:**
- React
- HTML (Vanilla JS)
- Vue (HTML 기반으로 적용)

**백엔드 (서버 SDK 우선 사용):**
- Node.js (Express, Nest 등) → `@portone/server-sdk`
- Python (FastAPI, Flask, Django 등) → `portone-server-sdk`
- JVM (Spring, Kotlin 등) → `io.portone:server-sdk`

서버 SDK가 지원되지 않는 언어의 경우에만 REST API 직접 호출 코드를 생성한다.

### Step 6: PG사 선택

MCP 서버를 통해 사용 가능한 PG사 목록을 조회하여 선택지를 제공한다.

**채널 조회 순서:**

1. **사용자 상점의 채널 조회 시도** (로그인 필요)
   ```
   mcp__portone__listStores
   → 상점 ID 획득

   mcp__portone__getChannelsOfStore
   - store: 획득한 상점 ID
   - fields: ["id", "pg", "name", "canV2"]
   ```

2. **공용 테스트 채널 조회** (상점 채널이 없거나 실패 시)
   ```
   mcp__portone__listSharedTestChannels
   - pgProviders: (선택) 특정 PG사만 필터링
   ```

3. **기본 PG사 목록 사용** (MCP 조회 실패 시 fallback)
   - toss (토스페이먼츠)
   - nice (나이스페이먼츠)
   - inicis (KG이니시스)
   - kcp (NHN KCP)
   - kakao (카카오페이)
   - naver (네이버페이)
   - tosspay (토스페이)
   - ksnet (KSNET)
   - smartro (스마트로)

**AskUserQuestion으로 PG사 선택:**
조회된 PG사 목록을 선택지로 구성하여 사용자에게 제시한다.
mcp__portone__getChannelsOfStore 로 V2 지원 여부(canV2)를 알 수 있으면 함께 표시하여 호환성 확인 가능하게 한다.

### Step 7: 예시 코드 조회

**V2의 경우 반드시 MCP 도구로 예시 코드 조회:**

프론트엔드:
```
mcp__portone__readPortoneV2FrontendCode
- framework: react 또는 html
- pg: 선택한 PG사
- pay_method: 결제 수단 (card, virtualAccount 등)
```

백엔드:
```
mcp__portone__readPortoneV2BackendCode
- framework: express, fastapi, flask, spring-kotlin
- pg: 선택한 PG사
- pay_method: 결제 수단
```

**V1의 경우:**
- mcp__portone__listPortoneDocs로 관련 문서 검색
- mcp__portone__readPortoneDoc으로 문서 내용 조회

### Step 8: 코드 생성

조회한 예시 코드를 프로젝트 구조에 맞게 수정하여 생성:

1. **프론트엔드 코드**
   - SDK 설치 안내 (`@portone/browser-sdk`)
   - 결제 요청 함수 작성
   - 결제 결과 처리 로직

2. **백엔드 코드 (서버 SDK 우선)**
   - 서버 SDK 설치 안내
     - Node.js: `npm install @portone/server-sdk`
     - Python: `pip install portone-server-sdk`
     - JVM: `implementation("io.portone:server-sdk:0.+")`
   - SDK 클라이언트 초기화 코드
   - 결제 검증 API 엔드포인트 (SDK 메서드 활용)
   - 웹훅 처리 (SDK의 웹훅 검증 기능 활용)

3. **환경 변수 설정**
   - `.env.example` 파일 생성
   - API 키 설정 안내

### Step 9: 추가 안내

생성된 코드와 함께 제공:
- 포트원 콘솔 설정 안내
- 테스트 방법
- 프로덕션 전환 체크리스트

## 중요 사항

- V2 API 호출 시 `Bearer` 대신 `PortOne` 인증 스킴 우선 사용
- API Secret은 절대 클라이언트에 노출하지 않음
- 결제 완료 후 반드시 서버에서 결제 검증 수행
- 웹훅 연동 권장 (네트워크 불안정 대비)

## 사용 예시

```
/generate-payment
/generate-payment v2
/generate-payment v2 checkout
/generate-payment v1 billing
```
