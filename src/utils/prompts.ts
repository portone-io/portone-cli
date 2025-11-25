import type { IntegrationOptions } from '../steps/run-integration.js';

export function getIntegrationPrompt(options: IntegrationOptions): string {
  const { type, version, pg, method } = options;

  if (type === 'payment') {
    return getPaymentPrompt(version, pg, method);
  }

  return getIdentityPrompt(version);
}

function getPaymentPrompt(version: 'v1' | 'v2', pg?: string, method?: string): string {
  const versionLabel = version.toUpperCase();

  return `포트원 ${versionLabel} 결제 연동을 진행해주세요.

${pg ? `PG사: ${pg}` : ''}
${method ? `결제수단: ${method}` : ''}

## 수행할 작업

1. **프로젝트 분석**
   - 현재 프로젝트 구조 파악 (프레임워크, 언어 등)
   - package.json 또는 관련 설정 파일 확인

2. **포트원 문서 참조**
   - portone MCP 서버의 readPortoneV2FrontendCode, readPortoneV2BackendCode 도구를 사용하여 공식 예시 코드 확인
   - listPortoneDocs, readPortoneDoc 도구로 필요한 문서 참조

3. **결제 연동 코드 생성**
   - 프론트엔드: 결제 요청 페이지/컴포넌트
   - 백엔드: 결제 검증 API 엔드포인트
   - 프로젝트 구조에 맞게 파일 생성

4. **환경 설정**
   - 필요한 npm 패키지 설치 (npm install 실행)
   - .env.example 또는 환경변수 설정 안내

5. **테스트 안내**
   - 테스트 방법 설명
   - 테스트용 카드번호 등 안내

## 주의사항
- API 키는 반드시 환경변수로 관리
- 에러 처리 포함
- 포트원 ${versionLabel} API 스펙 준수
- 실제 파일을 생성하고 수정해주세요
`;
}

function getIdentityPrompt(version: 'v1' | 'v2'): string {
  const versionLabel = version.toUpperCase();

  return `포트원 ${versionLabel} 본인인증 연동을 진행해주세요.

## 수행할 작업

1. **프로젝트 분석**
   - 현재 프로젝트 구조 파악
   - 사용 중인 프레임워크 확인

2. **포트원 문서 참조**
   - portone MCP 서버의 listPortoneDocs, readPortoneDoc 도구로 본인인증 관련 문서 참조
   - 본인인증 API 스펙 확인

3. **본인인증 연동 코드 생성**
   - 프론트엔드: 본인인증 요청 페이지/컴포넌트
   - 백엔드: 본인인증 결과 조회 API
   - 프로젝트 구조에 맞게 파일 생성

4. **환경 설정**
   - 필요한 npm 패키지 설치
   - 환경변수 설정 안내

5. **테스트 안내**
   - 테스트 환경 설정 방법
   - 본인인증 테스트 진행 방법

## 주의사항
- API 키는 반드시 환경변수로 관리
- 개인정보 보호 고려
- 포트원 ${versionLabel} API 스펙 준수
- 실제 파일을 생성하고 수정해주세요
`;
}
