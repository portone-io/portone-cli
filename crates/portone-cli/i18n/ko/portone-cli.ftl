core-config-read = 설정 파일을 읽지 못했습니다: {$path}

core-config-invalid = 잘못된 설정 파일입니다: {$path} ({$position})

core-config-directory = 설정 디렉터리를 만들지 못했습니다: {$path}

core-config-serialize = 설정을 직렬화하지 못했습니다

core-config-save = 설정 파일을 저장하지 못했습니다: {$path}

core-response-parse = 응답 본문을 해석하지 못했습니다: {$error}

core-jq-invalid = 잘못된 jq 필터입니다: {$error}

core-jq-render = jq 출력을 표시하지 못했습니다: {$error}

core-output-binary = 바이너리 내용을 터미널에 출력할 수 없습니다. stdout을 리디렉션하거나 파이프로 연결하거나 --allow-escape-sequences를 지정하세요

core-output-escapes = 응답에 터미널 이스케이프 시퀀스가 포함되어 있습니다. 출력하려면 --allow-escape-sequences를 지정하세요

core-field-invalid = 잘못된 키입니다: {$key}

core-field-value = 필드 {$key}에는 '=' 뒤에 값이 필요합니다

core-field-parse = {$key} 값을 해석하지 못했습니다

core-field-array = {$key}에는 배열이 필요하지만 {$actual} 형식이 있습니다

core-field-map = {$key}에는 객체가 필요하지만 {$actual} 형식이 있습니다

core-field-override = {$key}의 기존 필드를 덮어쓸 수 없습니다

core-field-open = 파일 읽기 {$path}
core-header-value = 헤더 {$header}에는 ':' 뒤에 값이 필요합니다

core-header-name = 잘못된 헤더 이름입니다: {$name}

core-content-length = 잘못된 Content-Length 값입니다: {$value}

core-input-stdin = 표준 입력에서 요청 본문을 읽지 못했습니다

core-input-file = {$path}에서 요청 본문을 읽지 못했습니다

core-http-method = 잘못된 HTTP 메서드입니다: {$method}

core-request-build = 요청을 구성하지 못했습니다

core-response-read = 응답 본문을 읽지 못했습니다

core-request-log = {$url}에 요청

core-body-omitted = {$bytes}바이트의 본문 생략

core-pagination-unknown = 페이지네이션 방식을 확인할 수 없어 첫 페이지에서 중단합니다

core-pagination-limit = 오프셋 페이지네이션 한도(60000)에 도달하여 중단합니다

core-pagination-cursor = 페이지네이션 커서가 이동하지 않아 중단합니다

core-pager-start = 페이저를 시작하지 못했습니다: {$error}

core-pager-invalid = 잘못된 페이저 명령입니다

core-pager-empty = 페이저 명령이 비어 있습니다

core-credentials-missing = 인증 정보를 찾을 수 없습니다. `portone auth login`을 실행하거나 PORTONE_ACCESS_TOKEN을 설정하세요

core-paginate-method = GET 이외의 요청에서는 `--paginate` 옵션을 사용할 수 없습니다

core-paginate-input = `--paginate` 옵션과 `--input`을 함께 사용할 수 없습니다

core-slurp-jq = `--slurp` 옵션과 `--jq`를 함께 사용할 수 없습니다

core-slurp-paginate = `--slurp`를 사용하려면 `--paginate`가 필요합니다

core-output-conflict = `--jq`, `--silent`, `--verbose` 중 하나만 사용할 수 있습니다

core-input-fields = `--input`과 필드 옵션을 함께 사용할 수 없습니다

help-about-portone = PortOne CLI

help-about-api = 인증 정보를 사용하여 PortOne V2 API 요청 전송

help-about-auth = PortOne 인증 관리

help-about-login = PortOne 콘솔 로그인

help-about-logout = 서버의 토큰을 취소하지 않고 로컬 인증 정보 삭제

help-about-status = 인증 상태 확인

help-about-token = 현재 콘솔 액세스 토큰 출력

help-about-setup = AI 코딩 도구용 PortOne 플러그인 설치

help-about-completion = 셸 자동완성 스크립트 생성

help-about-help = 이 메시지 또는 지정한 하위 명령어의 도움말 출력

help-about-payment = 결제 조회 및 관리
help-about-payment-list = 최근 결제 목록 조회
help-about-payment-view = 결제 상세 조회
help-about-payment-transactions = 결제 시도 내역 조회 (불안정 API)
help-about-payment-cancel = 결제 취소
help-about-payment-webhook = 결제 웹훅 조회 및 재발송
help-about-payment-webhook-list = 결제 웹훅 목록 조회
help-about-payment-webhook-resend = 결제 웹훅 재발송
help-about-store = 기본 상점 설정
help-about-store-set-default = 설정 프로필의 기본 상점 지정
help-store-id = 기본값으로 저장할 상점 ID
help-store-view = 저장된 기본 상점 ID 출력
help-store-unset = 저장된 기본 상점 설정 해제
help-payment-store = 상점 ID (기본값: PORTONE_STORE_ID 또는 프로필 store_id)
help-payment-id = 고객사가 지정한 결제 ID
help-resource-json = JSON 출력, 쉼표로 구분한 필드 선택 가능
help-resource-jq = jq 표현식으로 JSON 출력 필터링 (--json 필요)
help-payment-limit = 조회할 최대 결제 건수 (1-60000)
help-payment-status = 결제 상태로 필터링 (반복 또는 쉼표로 구분)
help-payment-method = 결제 수단으로 필터링 (반복 또는 쉼표로 구분)
help-payment-pg = PG사로 필터링 (반복 또는 쉼표로 구분)
help-payment-currency = KRW, USD 등의 통화 코드로 필터링
help-payment-test = 테스트 결제만 조회
help-payment-live = 실결제만 조회
help-payment-version = PortOne 결제 버전으로 필터링
help-payment-from = 조회 기간 시작 (기본값: --until 기준 90일 전)
help-payment-until = 조회 기간 종료 (기본값: 현재 시각)
help-payment-time-field = --from 및 --until에 사용할 시각 필드
help-payment-sort = 결제 정렬에 사용할 필드
help-payment-order = 정렬 순서
help-payment-search = 결제 텍스트 검색
help-payment-search-field = 검색할 결제 필드
help-payment-all-stores = 기본 상점 설정을 무시하고 접근 가능한 모든 상점 조회
help-payment-cancel-reason = 결제 취소 사유
help-payment-cancel-amount = 최소 통화 단위의 취소 금액 (기본값: 남은 금액 전체)
help-payment-cancel-tax-free = 최소 통화 단위의 면세 취소 금액
help-payment-cancel-vat = 최소 통화 단위의 부가세 취소 금액
help-payment-cancel-current = 최소 통화 단위의 예상 취소 가능 잔액
help-payment-cancel-input = 파일에서 취소 JSON 본문 읽기 (표준 입력은 -)
help-payment-cancel-yes = 확인 생략 (비대화형 실행 시 필수)
help-payment-webhook-id = 재발송할 웹훅 (기본값: 가장 최근 웹훅)

help-heading-commands = 명령어

help-heading-arguments = 인자

help-heading-options = 옵션

help-heading-usage = 사용법:

help-flag-help = 도움말 출력

help-flag-help-short = 도움말 출력 ('--help'로 자세히 보기)

help-flag-help-long = 도움말 출력 ('-h'로 요약 보기)

help-flag-version = 버전 출력

help-profile-store = 인증 정보를 저장할 설정 프로필

help-profile-remove = 삭제할 설정 프로필

help-profile-use = 사용할 설정 프로필

help-base-url = API 요청의 기본 URL (기본값: https://api.portone.io)

help-endpoint = 엔드포인트 경로, 전체 URL 또는 GraphQL API를 위한 graphql

help-method = 요청의 HTTP 메서드 (기본값: GET, 필드가 있으면 POST)

help-fields = key=value 형식으로 자료형을 변환하는 요청 필드 추가 (@path, @-, 정수, true, false, null 지원)

help-raw-fields = key=value 형식으로 문자열 요청 필드 추가

help-headers = key:value 형식으로 HTTP 요청 헤더 추가

help-input = 요청 본문으로 사용할 파일 (표준 입력은 "-" 사용)

help-include = 출력에 응답 상태 줄과 헤더 포함

help-paginate = 결과의 모든 페이지를 가져오도록 추가 요청 전송

help-slurp = 모든 페이지의 JSON 값을 하나의 배열로 출력 (--paginate 필요)

help-jq = jq 문법으로 응답 조회

help-cache = 3600s, 60m, 1h 등 지정한 시간 동안 응답 캐시

help-silent = 응답 본문 출력 생략

help-verbose = 출력에 전체 HTTP 요청과 응답 포함

help-allow-escape-sequences = 터미널 이스케이프 시퀀스 출력 허용

help-scopes = 요청할 콘솔 권한 범위 (쉼표로 구분, 기본값: HOME_AND_REPORT,TX_READ,CHANNEL_READ,STORE_READ,MERCHANT_READ)

help-insecure-storage = OS 키링 대신 설정 파일에 토큰 저장

help-no-browser = 브라우저를 열지 않고 로그인 URL 출력

help-show-secret = 액세스 토큰을 가리지 않고 표시

help-allow-dirty = Git 작업 트리에 커밋하지 않은 변경 사항이 있어도 진행

help-assistant = 설정할 AI 코딩 도구 (claude | codex | both)

help-shell = 자동완성 스크립트를 생성할 셸

help-subcommand = 하위 명령어의 도움말 출력

help-api-long-about =
    인증 정보를 사용하여 PortOne V2 API에 HTTP 요청을 보내고 응답을 출력합니다.

    `<ENDPOINT>` 인자에는 `/payments/{ "{" }paymentId{ "}" }` 같은 REST 경로
    (자리표시자를 실제 값으로 바꾸세요), 전체 URL 또는 GraphQL API를 위한
    `graphql`을 사용할 수 있습니다. 경로는 `--base-url`에 추가되며, 기본값은
    `https://api.portone.io`입니다. 전체 URL은 그대로 사용합니다. 전체 URL의
    출처(origin)가 기본 URL과 다르면 Authorization 헤더를 보내지 않습니다.

    기본 HTTP 메서드는 `GET`이며, 필드나 `--input`으로 요청 본문을 제공하면
    `POST`를 사용합니다. `--method`로 메서드를 지정할 수 있습니다. PortOne V2
    목록 엔드포인트는 GET 요청 본문으로 필터를 받으므로, 필드를 보내려면
    `-X GET`을 함께 지정하세요.

    `-f/--raw-field`에 `key=value` 형식의 값을 전달하면 문자열 필드를 추가합니다.
    `-F/--field`는 값에 따라 자료형을 변환합니다.

    - `true`, `false`, `null` 및 정수는 JSON 자료형으로 변환합니다.
    - `@`로 시작하는 값은 나머지 경로의 파일에서 읽고, `@-`는 표준 입력에서
      읽습니다.

    중첩 값에는 `key[subkey]=value`를, 배열에는 반복되는 `key[]=value` 필드를,
    빈 배열에는 값 없이 `key[]`를 사용합니다.

    GraphQL 요청에서는 `query`와 `operationName`을 제외한 모든 필드를
    GraphQL 변수로 보냅니다. 응답에 `errors` 배열이 있으면 HTTP 상태가
    200이어도 종료 코드 1을 반환합니다.

    미리 작성한 본문은 `--input <FILE>`로 전달하거나, `-`로 표준 입력에서
    읽을 수 있습니다. `--input`은 필드 옵션 또는 `--paginate`와 함께 사용할 수 없습니다.

    `--paginate`를 사용하면 더 이상 페이지가 없을 때까지 요청을 계속합니다.
    REST 페이지 이동은 오프셋 방식의 `page.totalCount` 또는 커서 방식의
    `items[].cursor`를 사용합니다. `page` 필드가 없으면 오프셋 페이지 이동은
    `number=0, size=100`에서 시작합니다. GraphQL 페이지 이동에는
    `$endCursor: String` 변수와 `pageInfo { "{" } hasNextPage endCursor { "}" }` 선택이
    필요합니다. 각 페이지를 별도의 JSON 값으로 출력하며, `--slurp`를 사용하면
    모든 페이지를 하나의 배열로 묶습니다.

    `-q/--jq`는 내장된 jaq 엔진을 사용하며 `--slurp`와 함께 사용할 수 없습니다.
    `--jq`, `--silent`, `--verbose` 중 하나만 사용할 수 있습니다.

    환경 변수:

    - `PORTONE_ACCESS_TOKEN`: 콘솔 액세스 토큰 (프로필보다 우선하며 갱신하지 않음)
    - `PORTONE_API_BASE`: API 기본 URL (`--base-url` > 환경 변수 > 프로필 > 기본값)
    - `PORTONE_CONFIG_DIR`: 설정 디렉터리
    - `PORTONE_CACHE_DIR`: `--cache`에서 사용하는 응답 캐시 디렉터리
    - `PORTONE_PAGER`, `PAGER`: TTY 출력용 페이저 (`cat` 또는 빈 값으로 비활성화)
    - `NO_COLOR`, `CLICOLOR_FORCE`: 색상 출력 제어

help-api-examples =
    # 결제 조회 (자리표시자를 실제 ID로 바꾸세요)
    $ portone api /payments/{ "{" }paymentId{ "}" }

    # 경로에 쿼리 매개변수 직접 추가
    $ portone api '/payments/{ "{" }paymentId{ "}" }?storeId=store-xxx'

    # GET 요청 본문에 필터를 넣어 결제 목록 조회
    $ portone api /payments -X GET -F 'page[size]=10' -F 'filter[isTest]=true'

    # 배열 필드 전달
    $ portone api /payments -X GET \
      -F 'filter[methods][]=CARD' -F 'filter[methods][]=EASY_PAY'

    # 결제 취소 (필드가 있으면 요청이 POST로 전환됨)
    $ portone api /payments/{ "{" }paymentId{ "}" }/cancel -f reason='Customer request'

    # 파일 또는 표준 입력에서 JSON 요청 본문 읽기
    $ portone api /payments/{ "{" }paymentId{ "}" }/cancel --input cancel.json
    $ echo '{ "{" }"reason":"Customer request"{ "}" }' | portone api /payments/{ "{" }paymentId{ "}" }/cancel --input -

    # 사용자 지정 헤더 추가
    $ portone api /payments/{ "{" }paymentId{ "}" }/cancel -f reason=duplicate-payment \
      -H 'Idempotency-Key: abc123'

    # 모든 페이지를 가져와 결제 ID 출력
    $ portone api /payments -X GET --paginate -q '.items[].id'

    # 커서 방식 엔드포인트의 모든 페이지를 하나의 JSON 배열로 출력
    $ portone api /payments-by-cursor -X GET --paginate --slurp

    # 응답을 한 시간 동안 캐시
    $ portone api /payments/{ "{" }paymentId{ "}" } --cache 1h

    # GraphQL 쿼리 전송
    $ portone api graphql \
      -f query='query { "{" } merchant { "{" } ... on Merchant { "{" } id plainId { "}" } { "}" } { "}" }'

    # GraphQL 변수 전달 (query를 제외한 모든 필드가 변수로 전달됨)
    $ portone api graphql -f id='<merchant-global-id>' -f query='
      query($id: ID!) { "{" } node(id: $id) { "{" } ... on Merchant { "{" } plainId { "}" } { "}" } { "}" }
    '

    # GraphQL 페이지 이동 및 중첩 필드로 객체 변수 구성
    $ portone api graphql --paginate --slurp \
      -f storeId='<store-global-id>' \
      -F 'filter[statuses][]=IN_PROGRESS' -F 'filter[cardCompanies][]' \
      -f query='
      query($storeId: ID!, $filter: PromotionFilterInput!, $endCursor: String) { "{" }
        node(id: $storeId) { "{" }
    { "      " }{ "..." } on Store { "{" }
            promotions(filter: $filter, first: 50, after: $endCursor) { "{" }
              edges { "{" } node { "{" } id name status { "}" } { "}" }
              pageInfo { "{" } hasNextPage endCursor { "}" }
    { "        " }{ "}" }
    { "      " }{ "}" }
    { "    " }{ "}" }
    { "  " }{ "}" }'

help-metadata-env = [환경 변수: { $value }]
help-metadata-default = [기본값: { $value }]
help-metadata-possible-values = [가능한 값: { $values }]

auth-invalid-redirect-uri = 잘못된 리디렉션 URI: { $uri }
auth-invalid-env-url = { $name } 값이 URL이 아닙니다: { $value }
auth-invalid-env-url-scheme = { $name } 값은 HTTP 또는 HTTPS URL이어야 합니다: { $value }
auth-random-failed = 난수 바이트를 생성하지 못했습니다: { $error }
auth-token-request-failed = 토큰 요청에 실패했습니다
auth-token-read-failed = 토큰 응답을 읽지 못했습니다
auth-token-parse-failed = 토큰 응답을 해석하지 못했습니다
auth-token-missing-access-token = 토큰 응답에 access_token이 없습니다
auth-token-missing-expires-in = 토큰 응답에 expires_in이 없습니다
auth-keyring-timeout = 키링이 { $seconds }초 이내에 응답하지 않았습니다
auth-stored-token-parse-failed = 저장된 토큰을 해석하지 못했습니다: { $error }
auth-refresh-lock-busy = 다른 portone 프로세스가 토큰을 갱신하고 있습니다. 잠시 후 다시 시도하세요
auth-config-lock-busy = 다른 portone 프로세스가 설정 파일을 갱신하고 있습니다. 잠시 후 다시 시도하세요
auth-lock-directory-failed = 잠금 디렉터리를 생성하지 못했습니다: { $path }
auth-lock-open-failed = 잠금 파일을 열지 못했습니다: { $path }
auth-lock-acquire-failed = 잠금을 획득하지 못했습니다: { $path }
auth-browser-invalid-url = 잘못된 URL { $url }: { $error }
auth-browser-unsupported-url = HTTP 또는 HTTPS가 아닌 URL은 열 수 없습니다: { $url }
auth-keyring-load-failed = 키링({ $service }/{ $id })에서 '{ $profile }' 프로필의 토큰을 읽지 못했습니다
auth-session-expired = 콘솔 로그인 세션이 만료되었습니다. `portone auth login`을 실행해 다시 인증하세요
auth-refresh-rejected = 토큰 갱신이 거부되었습니다({ $error }): { $detail }
auth-refresh-failed-continuing = portone: 토큰 갱신에 실패해 현재 토큰으로 계속 진행합니다: { $error }
auth-refresh-failed = 토큰 갱신에 실패했습니다
auth-refreshed-keyring-fallback = portone: 갱신한 토큰을 키링에 저장하지 못해 설정 파일에 저장합니다: { $error }
auth-refreshed-save-failed = 갱신한 토큰을 저장하지 못했습니다. 다음 실행 시 다시 인증해야 할 수 있습니다
auth-validation-request-failed = 콘솔 토큰 검증 요청에 실패했습니다
auth-validation-parse-failed = 콘솔 토큰 검증 응답을 해석하지 못했습니다
auth-callback-invalid-host = 리디렉션 URI의 호스트는 127.0.0.1 또는 localhost여야 합니다: { $uri }
auth-callback-missing-port = 리디렉션 URI에 포트가 없습니다: { $uri }
auth-callback-port-in-use = { $port }번 포트가 이미 사용 중입니다. 해당 포트를 사용하는 다른 portone 로그인 또는 MCP 서버를 종료한 후 다시 시도하세요
auth-callback-start-failed = 127.0.0.1:{ $port }에서 콜백 서버를 시작하지 못했습니다
auth-callback-timeout = 콘솔 로그인을 { $minutes }분 동안 기다렸지만 시간이 초과되었습니다
auth-callback-accept-failed = 콜백 연결을 수락하지 못했습니다
auth-callback-invalid-request = 잘못된 요청
auth-callback-method-not-allowed = 허용되지 않는 요청
auth-callback-not-found = 페이지를 찾을 수 없습니다
auth-callback-unverified-request = 로그인 요청을 확인할 수 없습니다
auth-callback-state-mismatch-detail = state 값이 일치하지 않습니다. 터미널에서 로그인을 다시 시작하세요.
auth-callback-state-mismatch = portone: state 값이 일치하지 않는 콜백을 무시했습니다
auth-callback-denied-title = 로그인이 거부되었습니다
auth-callback-denied-detail = 이 창을 닫고 터미널의 안내를 따르세요.
auth-callback-complete-title = 로그인 완료
auth-callback-complete-detail = 이 창을 닫고 터미널로 돌아가세요.
auth-callback-missing-code = code 값이 없습니다.
auth-callback-request-line-too-long = 요청 줄이 너무 깁니다
auth-callback-connection-closed = 요청 줄을 수신하기 전에 연결이 닫혔습니다
auth-login-env-active = portone: { $name } 환경 변수로 인증하고 있습니다. 로그인 인증 정보를 저장하려면 먼저 이 환경 변수를 해제하세요
auth-login-browser-instructions = 브라우저에서 콘솔 로그인을 완료하세요. 브라우저가 열리지 않았다면 다음 URL로 이동하세요:
auth-login-browser-failed = portone: 브라우저를 열지 못했습니다: { $error }
auth-login-denied = 콘솔 로그인이 거부되었습니다: { $error }{ $detail }
auth-login-token-failed = 토큰을 받지 못했습니다
auth-login-missing-scopes = portone: 요청한 권한 중 일부가 부여되지 않았습니다: { $scopes }
auth-login-environment-mismatch = 발급된 토큰으로 { $base_url }에 접근하지 못했습니다. 콘솔과 API 환경이 일치하는지 확인하세요
auth-login-complete = 콘솔 로그인 완료 (고객사 { $merchant })
auth-login-stored = '{ $profile }' 프로필에 콘솔 로그인 인증 정보를 저장했습니다.
auth-source-keyring = 키링({ $service }/{ $id })
auth-storage-file = 설정 파일(평문)
auth-storage = 저장 위치: { $location }
auth-login-keyring-timeout = 키링이 30초 이내에 응답하지 않았습니다({ $service }/{ $id }). 키링을 확인한 후 다시 시도하세요
auth-login-keyring-fallback = portone: 키링을 사용할 수 없어 토큰을 설정 파일에 저장합니다: { $error }
auth-login-cleanup-failed = portone: 이전 콘솔 로그인 토큰을 삭제하지 못했습니다({ $service }/{ $id }): { $error }
auth-logout-env-active = portone: { $name } 환경 변수로 인증하고 있습니다. 저장된 인증 정보를 삭제하려면 먼저 이 환경 변수를 해제하세요
auth-profile-not-found = '{ $profile }' 프로필이 없습니다
auth-logout-keyring-delete-failed = 키링({ $service }/{ $id })에서 토큰을 삭제하지 못했습니다
auth-logout-removed = '{ $profile }' 프로필을 삭제했습니다.
auth-no-credentials = portone: 저장된 인증 정보를 찾을 수 없습니다. `portone auth login`을 실행해 인증하세요
auth-source-environment = 환경 변수 { $name }
auth-source-config = 설정 프로필 '{ $profile }'
auth-status-authentication = 인증 방식: 콘솔 OAuth
auth-status-source = 출처: { $source }
auth-status-access-token = 액세스 토큰: { $token }
auth-status-expires = 만료 시각: { $timestamp } ({ $remaining })
auth-status-session-expires = 세션 만료 시각: { $timestamp }
auth-status-scopes = 권한: { $scopes }
auth-status-issued-by = 발급 주체: { $client_id } @ { $url }
auth-status-api-base-url = API 기본 URL: { $url }
auth-status-valid = 검증 결과: 유효함 (상점 { $merchant })
auth-status-invalid = 검증 결과: 유효하지 않음
auth-status-invalid-token = portone: 콘솔 토큰이 유효하지 않습니다. `portone auth login`을 실행해 다시 인증하세요
auth-remaining-expired = 만료됨
auth-remaining-hours = { $hours }시간 { $minutes }분 남음
auth-remaining-minutes = { $minutes }분 남음
auth-remaining-seconds = { $seconds }초 남음

setup-starting = 🚀 PortOne 연동 설정을 시작합니다
setup-check-git = Git 상태 확인 중...
setup-git-dirty = Git 작업 트리에 커밋하지 않은 변경 사항이 있습니다
setup-allow-dirty-hint = 계속하려면 변경 사항을 커밋하거나 --allow-dirty를 사용하세요
setup-git-checked = Git 상태 확인 완료
setup-check-installation = { $assistant } 설치 확인 중...
setup-not-installed = { $assistant }가 설치되어 있지 않습니다
setup-install-question = { $assistant }를 설치할까요?
setup-installing = { $assistant } 설치 중...
setup-installed = { $assistant } 설치 완료
setup-install-failed = { $assistant } 설치 실패
setup-install-manually = { $assistant }를 직접 설치하세요: { $command }
setup-installation-found = { $assistant } 설치 확인 완료
setup-updating = { $assistant } 업데이트 중...
setup-updated = { $assistant } 업데이트 완료
setup-update-failed = { $assistant } 업데이트 실패 (계속 진행합니다)
setup-configuring-plugin = { $assistant }용 PortOne 플러그인 설정 중...
setup-plugin-configured = { $assistant }용 플러그인 설정 완료
setup-plugin-failed = { $assistant }용 플러그인 설정 실패
setup-complete = ✅ 설정이 완료되었습니다!
setup-unsupported-assistant = 지원하지 않는 어시스턴트: { $assistant }
setup-assistant-required = 비대화형 환경에서는 --assistant가 필요합니다 (claude | codex | both)
setup-assistant-question = 어떤 어시스턴트를 설정할까요?
setup-selection-hint = ↑↓로 이동, Enter로 선택, 입력하여 필터링
setup-prompt-canceled-indicator = <취소됨>
setup-confirm-invalid-answer = 잘못된 응답입니다. 동의하면 'y', 거절하면 'n'을 입력하세요
setup-confirm-yes = 예
setup-confirm-no = 아니요
setup-prompt-not-tty = 입력 장치가 TTY가 아닙니다
setup-prompt-canceled = 사용자가 작업을 취소했습니다
setup-prompt-interrupted = 사용자가 작업을 중단했습니다
setup-prompt-invalid-config = 질문 설정이 올바르지 않습니다: { $detail }
setup-prompt-io-error = 입출력 오류
setup-prompt-custom-error = 사용자 정의 오류
setup-next-steps = 📋 다음 단계
setup-start-assistant = 1. { $assistant }를 시작하세요:
setup-run-slash-command = 2. 다음 슬래시 명령을 실행하세요:
setup-codex-prompts = 2. `portone-codex` 플러그인을 설치한 상태에서 다음과 같이 요청해 보세요:
setup-example-implement = PortOne V2 일회성 결제 연동을 구현해 줘
setup-example-review = 이 프로젝트의 PortOne 연동을 검토해 줘
setup-command-run-failed = 명령 실행 실패: { $command }
setup-command-output-failed = 명령 실패: { $command }
    { $output }
setup-command-failed = 명령 실패: { $command }
setup-create-directory-failed = 디렉터리 생성 실패: { $path }
setup-extract-assets-failed = 플러그인 파일 추출 실패: { $path }
setup-read-marketplace-failed = marketplace.json 읽기 실패: { $path }
setup-write-marketplace-failed = marketplace.json 쓰기 실패: { $path }
setup-parse-marketplace-failed = marketplace.json 구문 분석 실패

resource-invalid-base-url = API 기본 URL은 HTTP 또는 HTTPS URL이어야 합니다
resource-invalid-path = 결제 ID로 . 또는 ..을 사용하거나 제어문자를 포함할 수 없습니다.
resource-http-error = HTTP { $status }: { $detail }
resource-jq-requires-json = --jq를 사용하려면 --json이 필요합니다
resource-json-field = 알 수 없는 JSON 필드 '{ $field }'입니다. 가능한 필드: { $fields }
resource-status-requested = 요청 접수
resource-status-succeeded = 완료
resource-status-failed = 실패
resource-status-ready = 결제 준비
resource-status-pending = 결제 대기
resource-status-virtual-account = 가상계좌 발급
resource-status-paid = 결제 완료
resource-status-partial-cancelled = 부분 취소
resource-status-cancelled = 취소
resource-no-results = 조회 결과가 없습니다.
resource-label-id = ID
resource-label-status = 상태
resource-label-amount = 금액(최소 통화 단위)
resource-label-reason = 사유
resource-label-requested = 요청 시각
resource-label-cancelled-at = 취소 시각
resource-label-receipt = 영수증
resource-label-updated = 상태 변경 시각
resource-label-mode = 테스트/실결제
resource-label-order = 주문명
resource-label-pg-tx = PG 거래 ID
resource-label-failure = 실패 사유
resource-label-url = URL
resource-label-attempts = 발송 횟수
resource-label-triggered = 발송 요청 시각
resource-label-store = 상점
resource-label-version = 버전
resource-label-method = 결제 수단
resource-label-channel = 채널
resource-label-transaction = 거래 ID
resource-label-paid = 결제 시각
resource-label-cancelled-amount = 취소 금액(최소 통화 단위)
resource-label-pg-code = PG 코드
resource-label-pg-message = PG 메시지
resource-label-bank = 은행
resource-label-account = 계좌 번호
resource-label-account-holder = 예금주
resource-label-expires = 만료 시각
resource-label-cancellations = 취소 내역
resource-label-webhooks = 웹훅 내역

payment-empty-value = { $field } 값은 비어 있을 수 없습니다
payment-response-field = API 응답에 올바른 { $field } 필드가 없습니다
payment-search-field-requires-search = --search-field를 사용하려면 --search가 필요합니다
payment-list-all-stores = 접근 가능한 전체 상점
payment-list-api-default = API 기본 상점 범위
payment-list-scope = { $store } | { $version } | { $environment } | { $from } ~ { $until }
payment-date-range = 올바른 조회 기간을 지정해야 하며 --from은 --until보다 늦을 수 없습니다
payment-date-invalid = { $field } 값은 RFC3339 시각이어야 합니다: { $value }
payment-cancel-needs-yes = 비대화형 환경에서 결제를 취소하려면 --yes가 필요합니다
payment-cancel-failed = portone: 결제 취소에 실패했습니다
payment-cancel-input-json = 잘못된 취소 JSON입니다: { $error }
payment-cancel-input-amount = { $field }는 최소 통화 단위의 정수여야 합니다 (amount는 양수, 다른 금액은 0 이상)
payment-cancel-input-field = 취소 필드가 없거나 올바르지 않습니다: { $field }
payment-cancel-store-conflict = --store 값이 취소 입력의 storeId와 다릅니다
payment-cancel-all-remaining = 남은 금액 전체
payment-cancel-preview =
    프로필: { $profile }
    상점: { $store }
    결제: { $payment_id }
    테스트/실결제: { $environment }
    취소 금액(최소 통화 단위): { $amount } { $currency }
    사유: { $reason }
payment-cancel-confirm = 이 결제를 취소하시겠습니까? [y/N]
payment-cancel-aborted = 결제 취소 요청을 중단했습니다.
payment-webhook-failed = portone: 웹훅 전달에 실패했습니다

store-discovery-request-failed = 접근 가능한 상점 목록을 요청하지 못했습니다
store-discovery-parse-failed = 상점 조회 응답을 해석하지 못했습니다
store-discovery-http = 상점 조회에 실패했습니다 (HTTP { $status })
store-discovery-graphql = 상점 조회에 실패했습니다: { $detail }
store-discovery-union = 상점 조회 결과가 { $kind }입니다: { $detail }
store-discovery-malformed = 상점 조회 응답 형식이 올바르지 않습니다
store-selection-question = 기본으로 사용할 상점을 선택하세요
store-selection-hint = 방향키로 이동, Enter로 선택, 입력하여 검색
store-selection-skip = 나중에 설정
store-selection-representative = 대표
store-selection-canceled = <취소됨>
store-selection-failed = 기본 상점을 선택하지 못했습니다
store-selection-empty = 접근 가능한 상점이 없습니다
store-selection-requires-tty = 비대화형 환경에서는 상점 ID를 지정해야 합니다
store-default-set = 프로필 '{ $profile }'의 기본 상점을 { $store }로 설정했습니다.
store-default-unset = 프로필 '{ $profile }'의 기본 상점 설정을 해제했습니다.
store-default-missing = 프로필 '{ $profile }'에 기본 상점이 설정되어 있지 않습니다
store-invalid-id = 상점 ID는 비어 있거나 제어 문자를 포함할 수 없습니다
auth-login-store-unavailable = portone: 로그인했지만 기본 상점을 확인하지 못했습니다: { $error }
auth-login-store-selected = 기본 상점: { $store }
auth-login-store-unset = 기본 상점이 설정되지 않았습니다. 나중에 `portone store set-default`로 설정하세요.
