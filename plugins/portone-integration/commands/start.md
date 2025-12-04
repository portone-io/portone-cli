---
description: 포트원 결제 연동 코드를 대화형으로 생성합니다
argument-hint: 결제 연동 요구사항...
allowed-tools: ["Read", "Write", "Grep", "Glob", "Bash", "TodoWrite", "AskUserQuestion", "Task"]
---

# 포트원 결제 연동 코드 생성 워크플로우

사용자의 프로젝트에 맞는 포트원 결제 연동 코드를 대화형으로 생성합니다.

**전달된 인자:** $ARGUMENTS

---

## Core Principles

- **MCP 도구 필수 사용**: 포트원 관련 코드 작성 시 반드시 MCP 도구(mcp__portone__)를 통해 공식 예시 코드와 문서를 참조합니다.
- **버전 확인 우선**: V1과 V2는 완전히 다른 API이므로, 반드시 버전을 먼저 확정합니다.
- **Use specialized agents**: 코드 생성에는 payment-code-generator 에이전트를, 코드 검증에는 integration-validator 에이전트를 사용합니다.
- **보안 최우선**: API Secret은 절대 클라이언트 코드에 노출되지 않도록 합니다.
- **프로젝트 맞춤형**: 사용자의 프레임워크와 코딩 스타일에 맞게 코드를 생성합니다.
- **TodoWrite 활용**: 각 단계의 진행 상황을 추적합니다.

---

## Phase 1: 버전 및 결제 유형 확인

### 1.1 버전 선택

AskUserQuestion을 사용하여 물어봅니다:

```
질문: "어떤 포트원 버전을 사용하시겠습니까?"
옵션:
- V2 (권장): 최신 버전, 신규 프로젝트에 권장
- V1: 레거시 버전, 기존 연동 유지보수용
```

### 1.2 결제 유형 선택

AskUserQuestion을 사용하여 물어봅니다:

```
질문: "어떤 결제 유형을 연동하시겠습니까?"
옵션:
- 일반결제 (checkout): PG 결제창을 통한 일회성 결제
- 정기결제 (billing): 빌링키 발급 후 정기 과금
- 수기결제 (keyin): 카드 정보 직접 입력 (V2 전용)
- 본인인증 (identity): 휴대폰/PASS 본인인증
```

**주의**: 수기결제(keyin)는 V2에서만 별도 계약을 한 고객사에게만 지원됩니다. V1을 선택한 경우 이 옵션을 제외합니다.

### 1.3 PG사 선택

AskUserQuestion을 사용하여 물어봅니다.

버전에 따라 지원되는 PG사가 다릅니다:

**V2 지원 PG사:**
```
- 토스페이먼츠 (toss)
- 나이스페이먼츠 (nice)
- KG이니시스 (inicis)
- NHN KCP (kcp)
- 스마트로 (smartro)
- KSNET (ksnet)
- 한국결제네트웍스 (kpn)
- 웰컴페이먼츠 (welcome)
- 카카오페이 (kakao)
- 네이버페이 (naver)
- 토스페이 (tosspay)
- 하이픈 (hyphen)
- 엑심베이 (eximbay)
- 페이팔 (paypal)
```

**V1 지원 PG사:**
```
- 토스페이먼츠 (toss)
- 나이스페이먼츠 (nice)
- KG이니시스 (inicis)
- NHN KCP (kcp)
- 스마트로 (smartro)
- KSNET (ksnet)
- 헥토파이낸셜 (settle)
- 웰컴페이먼츠 (welcome)
- 키움페이 (daou)
- 이지페이 (kicc)
- 다날 (danal)
- 카카오페이 (kakao)
- 네이버페이 (naver)
- 토스페이 (tosspay)
- 페이코 (payco)
- 엑심베이 (eximbay)
- 페이먼트월 (paymentwall)
- 페이팔 (paypal)
```

AskUserQuestion 예시:
```
질문: "어떤 PG사를 사용하시겠습니까?"
옵션: (버전에 따라 위 목록에서 주요 PG사 4개 선택)
- 토스페이먼츠
- 나이스페이먼츠
- KG이니시스
- 기타: 다른 PG사 선택
```

**기타 선택 시** 전체 목록을 보여주고 추가 질문을 합니다.

---

## Phase 2: 프로젝트 환경 분석

버전과 결제 유형이 확정되면, 프로젝트 환경을 파악합니다:

1. **프로젝트 파일 확인**
   - package.json, requirements.txt, build.gradle 등 확인
   - 프론트엔드 프레임워크 감지 (React, Vue, Next.js, HTML 등)
   - 백엔드 프레임워크 감지 (Express, FastAPI, Spring, Flask 등)

2. **기존 결제 코드 확인**
   - 이미 포트원 연동이 되어 있는지 검색
   - 기존 V1 코드가 있다면 마이그레이션 필요 여부 확인

---

## Phase 3: 코드 생성 에이전트 호출

프로젝트 분석이 완료되면 **payment-code-generator** 에이전트를 호출합니다.

Task 도구를 사용하여 다음 정보와 함께 에이전트를 호출합니다:

```
subagent_type: "portone-integration:payment-code-generator"
prompt: |
  사용자 프로젝트에 포트원 결제 연동 코드를 생성해주세요.

  ## 확정된 정보
  - 포트원 버전: [V1 또는 V2]
  - 결제 유형: [checkout/billing/keyin/identity]
  - PG사: [선택된 PG사]

  ## 프로젝트 환경
  - 프론트엔드: [감지된 프레임워크]
  - 백엔드: [감지된 프레임워크]
  - TypeScript 사용: [예/아니오]

  ## 요청 사항
  1. MCP 도구로 공식 예시 코드 조회 (선택된 PG사 기준)
  2. 프로젝트 환경에 맞게 코드 커스터마이징
  3. 필요한 파일들 생성
  4. 환경 변수 설정 안내
```

---

## Phase 4: 코드 검증 에이전트 호출

코드 생성이 완료되면 **integration-validator** 에이전트를 호출하여 검증합니다.

Task 도구를 사용하여 에이전트를 호출합니다:

```
subagent_type: "portone-integration:integration-validator"
prompt: |
  방금 생성된 포트원 결제 연동 코드를 검증해주세요.

  ## 검증 대상
  - 포트원 버전: [V1 또는 V2]
  - 결제 유형: [checkout/billing/keyin/identity]
  - PG사: [선택된 PG사]

  ## 검증 항목
  1. SDK/API 사용법이 공식 문서와 일치하는지
  2. 필수 파라미터가 모두 포함되어 있는지
  3. 선택한 PG사에 맞는 channelKey 설정
  4. 보안 설정이 올바른지 (API Secret 노출 여부 등)
  5. 환경 변수 설정이 적절한지

  검증 결과를 리포트 형식으로 제공하고,
  문제가 있다면 구체적인 수정 방법을 안내해주세요.
```

---

## Phase 5: 최종 안내

모든 과정이 완료되면 사용자에게 다음을 안내합니다:

### 5.1 생성된 파일 요약
- 생성된 파일 목록과 각 파일의 역할

### 5.2 설정 가이드
- 포트원 콘솔에서 해야 할 설정
- 환경 변수 설정 방법
- 테스트 환경 구성 방법

### 5.3 테스트 방법
- 테스트 결제 수행 방법
- 디버깅 팁

### 5.4 프로덕션 체크리스트
- 실 운영 전 확인해야 할 사항들

---

## 워크플로우 실행

위 단계를 순차적으로 실행합니다:

1. **Phase 1** 시작: 인자를 확인하고, 없으면 AskUserQuestion으로 버전, 결제 유형, PG사를 물어봅니다.
2. **Phase 2**: 프로젝트 환경을 분석합니다.
3. **Phase 3**: payment-code-generator 에이전트로 코드를 생성합니다.
4. **Phase 4**: integration-validator 에이전트로 코드를 검증합니다.
5. **Phase 5**: 최종 안내를 제공합니다.

---

**지금 Phase 1부터 시작하세요.**
