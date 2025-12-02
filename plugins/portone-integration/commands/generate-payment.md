---
description: 포트원 결제 연동 코드를 대화형으로 생성합니다
argument-hint: [옵션: v1|v2] [결제유형: checkout|billing|keyin|identity]
allowed-tools: Read, Write, Glob, Grep, AskUserQuestion, mcp__portone__readPortoneV2FrontendCode, mcp__portone__readPortoneV2BackendCode, mcp__portone__readPortoneOpenapiSchema, mcp__portone__readPortoneOpenapiSchemaSummary, mcp__portone__listPortoneDocs, mcp__portone__readPortoneDoc, mcp__portone__regex_search_portone_docs
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

**백엔드:**
- Express (Node.js)
- FastAPI (Python)
- Flask (Python)
- Spring/Kotlin

### Step 6: PG사 선택 (선택사항)

테스트 환경이거나 특정 PG사가 없으면 기본값 사용.
주요 PG사: toss, nice, inicis, kcp, kakaopay, naverpay

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

2. **백엔드 코드**
   - SDK 설치 안내 (해당하는 경우)
   - 결제 검증 API 엔드포인트
   - 웹훅 처리 (선택)

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
