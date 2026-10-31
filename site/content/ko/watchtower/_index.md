---
title: "watchtower-rs"
site_name: "watchtower-rs"
description: "Rust로 만든 changedetection.io 호환 웹 페이지 감시 도구. 단일 실행 파일, 5,000개 감시에 약 24 MB."
tagline: "페이지의 변화를 조용히 지켜봅니다."
lead: "watchtower-rs는 웹 페이지를 정기적으로 확인해 달라진 점을 알려 주는 감시 도구입니다. changedetection.io와 호환되는 REST API를 갖추고, Rust 실행 파일 하나로 동작합니다."
pills: ["단일 실행 파일", "SQLite", "changedetection.io 호환", "REST API"]
nav_all: "앱 목록"
nav_features: "특징"
nav_start: "사용법"
nav_get: "다운로드"
nav_status: "개발 상황"
stats_title: "메모리 사용량(Windows 릴리스 빌드 실측)"
stats:
  - { value: "약 17 MB", label: "1,000개를 등록하고 대기 중일 때" }
  - { value: "약 24 MB", label: "5,000개를 300초 간격으로 감시할 때의 최대치" }
  - { value: "약 90초", label: "5,000개의 첫 가져오기가 모두 끝날 때까지" }
stats_note: "실행 파일은 약 28 MB입니다. 직접 만든 모의 사이트로 측정했으며 환경에 따라 달라집니다."
features_title: "특징"
features:
  - { icon: "🍃", title: "적은 메모리", body: "감시 항목을 많이 등록해도 메모리가 잘 늘지 않습니다. 스냅샷은 압축해 파일로 저장합니다." }
  - { icon: "🔁", title: "원본과 호환", body: "REST API는 x-api-key로 인증하는 원본 형태를 따르며, 원본의 url-watches.json도 가져올 수 있습니다." }
  - { icon: "✂️", title: "필요한 부분만 비교", body: "CSS 선택자, XPath, JSONPath, jq로 대상을 좁히고, 정규식과 무시할 문자열로 잡음을 없앱니다. 트리거 텍스트가 나타날 때만 알릴 수도 있습니다." }
  - { icon: "🌐", title: "JavaScript 페이지도", body: "따로 실행한 Chrome에 연결해 렌더링된 페이지를 가져옵니다. 클릭·입력·대기 단계를 등록하거나, 스크린샷에서 요소를 고르는 비주얼 셀렉터를 쓸 수 있습니다." }
  - { icon: "🏷️", title: "가격·재고·PDF", body: "상품 페이지의 구조화 데이터에서 가격을 읽어 기준 이하가 되면 알립니다. PDF 본문도 비교할 수 있습니다." }
  - { icon: "🔔", title: "다양한 알림 대상", body: "Discord, Slack, Telegram, ntfy, Gotify, Mattermost, Pushover, 이메일, Webhook을 지원하고, 그 밖의 서비스는 Apprise API로 전달합니다. 제목과 본문은 템플릿으로 바꿀 수 있습니다." }
  - { icon: "🛠️", title: "세밀한 가져오기 설정", body: "감시 항목별, 태그별, 전체로 헤더·메서드·본문·프록시·간격을 정할 수 있고, 가까운 설정이 우선합니다." }
  - { icon: "🛡️", title: "안전을 고려", body: "내부 주소 접속을 막는 SSRF 대책, argon2 비밀번호, CSRF 대책, Host 이름 허용 목록. 헤더 값과 알림 비밀 정보는 로그에 남기지 않습니다." }
  - { icon: "📈", title: "운영하기 쉬움", body: "OpenAPI 문서, Prometheus 형식의 /metrics, RSS 피드, 백업과 복원, 상태 확인을 제공하며 Docker 파일도 들어 있습니다." }
  - { icon: "📊", title: "대시보드", body: "감시 수, 실패 중인 항목, 알림 대기열, 최근 변화를 한 화면에서 볼 수 있습니다." }
  - { icon: "🚀", title: "명령 하나로 시작", body: "watchtower ui는 서버를 시작하고 브라우저에서 관리 화면을 엽니다." }
how_title: "동작 방식"
how:
  - { title: "가져오기", body: "정한 간격으로 페이지를 가져옵니다. 내부 주소 접속은 기본적으로 거부합니다." }
  - { title: "다듬기", body: "대상을 좁히고 불필요한 요소와 자주 바뀌는 문자를 없애 텍스트로 만듭니다." }
  - { title: "비교하고 알리기", body: "지난번과 달라지면 저장하고, 조건에 맞을 때만 알림을 보냅니다. 보내지 못한 알림은 다시 시도합니다." }
start_title: "사용법"
start_lead: "명령줄에서 비밀번호를 정하고 실행합니다. 관리 화면은 기본적으로 127.0.0.1:5000에서 열립니다."
start_code: |-
  watchtower set-password     # 관리 화면 비밀번호(표준 입력에서 한 줄)
  watchtower api-key          # REST API 키 발급
  watchtower watch add https://example.com/ --interval 300
  watchtower ui               # 시작하고 브라우저에서 관리 화면 열기(run은 화면을 열지 않음)
start_note: "환경 변수(WATCHTOWER_DATA_DIR, WATCHTOWER_LISTEN, WATCHTOWER_BASE_URL 등)나 설정 파일로 바꿉니다."
download_title: "다운로드"
download_button: "Google 드라이브에서 다운로드"
download_body: "Windows 11(64비트)용 zip이며 크기는 약 11 MB입니다. 압축을 풀고 watchtower.exe를 실행하면 되고, 설치 과정은 없습니다. Linux(x86_64, aarch64)용 정적 바이너리는 GitHub 릴리스 페이지에 있습니다."
cta_source: "소스 코드"
sha_label: "SHA-256(zip)"
release_label: "Linux 바이너리와 소스 코드(GitHub)"
download_url: "https://drive.google.com/drive/folders/1349ZQeTtuH27DqxitP2U3XoAaON8HTG0?usp=sharing"
repo_url: "https://github.com/taotaro1104-commits/watchtower-rs"
release_url: "https://github.com/taotaro1104-commits/watchtower-rs/releases"
download_file: "watchtower-rs-0.1.0-windows-x64.zip"
download_sha: "398abd5f1c571416bcd61d00f63711b1313fffbf5022b766fa0ba8810a10e3a9"
status_title: "개발 상황"
status_body: "개발 중입니다(v0.1.0, 프리릴리스). Windows 11에서 실제 바이너리를 실행하고 브라우저로 조작해 확인했습니다. 소스 코드는 GitHub에 공개되어 있습니다."
todo_title: "아직 확인하지 못한 것"
todo:
  - "Docker 이미지(CI에서 빌드는 되지만 아직 실행해 보지 않음)"
  - "Linux 정적 바이너리(musl)와 aarch64(빌드는 되지만 아직 실행해 보지 않음)"
  - "TLS 또는 STARTTLS를 쓰는 실제 메일 서버로의 전송"
footer: "© watchtower-rs — Apache-2.0"
---
