---
name: payment-code-generator
description: Use this skill when the user asks to implement PortOne payment integration code, add checkout or billing flows, migrate to PortOne V2, or generate payment-related backend/frontend changes. It should inspect the project first, confirm the required PortOne version and payment type, then generate project-specific code using the PortOne MCP server as the source of truth.
---

# PortOne Payment Code Generator

PortOne 결제 연동 코드를 생성할 때 사용하는 스킬이다. 사용자 요구사항을 바로 구현하지 말고, 먼저 프로젝트 구조와 기존 연동 여부를 파악한 뒤 PortOne MCP 서버에서 최신 예시와 문서를 조회해 맞춤형 코드를 작성한다.

## Goals

- 프로젝트의 프론트엔드/백엔드 환경을 파악한다.
- PortOne 버전(V1 또는 V2)과 결제 유형을 확정한다.
- PortOne 공식 예시와 문서를 기준으로 코드를 생성한다.
- 보안 요구사항과 서버 검증 흐름을 함께 반영한다.

## Workflow

1. 저장소를 탐색해 프레임워크, 언어, 기존 결제 코드, 환경 변수 패턴을 확인한다.
2. 버전이나 결제 유형이 불명확하면 꼭 필요한 최소 질문만 한다.
3. V2 구현 시 PortOne MCP 서버에서 프론트엔드/백엔드 예시 코드를 먼저 조회한다.
4. 프로젝트 구조에 맞는 파일 위치와 코딩 스타일로 구현한다.
5. 결제 완료 후 서버 검증, 에러 처리, 환경 변수 설정까지 함께 반영한다.
6. 변경 후 테스트 방법과 필요한 설치 명령을 사용자에게 알려준다.

## Required Checks

- API Secret은 클라이언트 코드에 두지 않는다.
- 결제 성공 응답만 믿지 말고 서버에서 결제를 검증한다.
- V2 백엔드는 가능하면 최신 PortOne Server SDK를 우선 사용한다.
- `.env` 계열 파일은 커밋 대상에서 제외한다.
- PG사 미지정 시 예시는 토스페이먼츠를 기본값으로 사용하되, 기존 코드에 맞는 PG가 있으면 그쪽을 우선한다.

## PortOne MCP Usage

- V2 프론트엔드 코드는 PortOne MCP의 V2 예시 코드를 먼저 확인한다.
- V2 백엔드 검증 및 웹훅은 Server SDK 문서와 예시를 우선 참조한다.
- V1은 관련 문서와 검색 도구를 사용해 정확한 파라미터를 확인한다.

## Output Expectations

- 생성한 코드가 프로젝트에 바로 붙을 수 있어야 한다.
- 파일 경로, 필요한 패키지 설치 명령, 환경 변수 이름을 명확히 제시한다.
- 테스트 결제나 검증 절차를 짧게 안내한다.
