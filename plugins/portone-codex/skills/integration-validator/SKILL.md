---
name: PortOne Integration Validator
description: Use this skill when the user asks to review, validate, or troubleshoot PortOne payment integration code. It should compare the implementation against PortOne official examples and docs, identify concrete bugs or compliance issues, and provide file-specific fixes.
---

# PortOne Integration Validator

PortOne 연동 코드가 공식 문서와 SDK 기준에 맞는지 검증하는 스킬이다. 단순한 스타일 피드백보다 동작 오류, API 오용, 보안 문제, 환경 설정 누락을 우선 찾아낸다.

## Goals

- 최근 수정된 PortOne 관련 코드를 찾는다.
- PortOne 버전과 결제 유형을 식별한다.
- 공식 예시와 현재 구현을 비교해 반드시 고쳐야 하는 문제를 찾는다.
- 환경 변수, 검증 로직, 웹훅 처리, SDK 사용법을 점검한다.

## Validation Flow

1. 저장소에서 PortOne 관련 파일과 의존성을 찾는다.
2. V1/V2, checkout/billing/keyin/identity 여부를 판별한다.
3. PortOne MCP 서버에서 해당 버전에 맞는 예시 코드와 문서를 조회한다.
4. 프론트엔드 요청 파라미터, 백엔드 검증, 인증 방식, 결제 상태 확인 로직을 대조한다.
5. 문제를 심각도 순서로 정리하고 구체적인 수정 방향을 제시한다.

## Must-Fix Checks

- V2 인증 헤더가 잘못된 스킴을 쓰지 않는지 확인한다.
- 결제 완료 후 서버에서 금액과 상태를 검증하는지 확인한다.
- API Secret 또는 채널 비밀값이 클라이언트에 노출되지 않는지 확인한다.
- 웹훅이 있다면 서명 검증 또는 중복 처리 방어가 있는지 확인한다.
- V1/V2 패턴이 혼재되어 있으면 치명적 문제로 취급한다.

## Reporting Style

- 이슈를 심각도 높은 순서로 제시한다.
- 가능하면 파일 경로와 라인 기준으로 설명한다.
- 기대 동작과 실제 구현의 차이를 짧게 설명한다.
- 문제 없으면 그렇게 명시하고, 남아 있는 테스트 공백만 따로 적는다.
