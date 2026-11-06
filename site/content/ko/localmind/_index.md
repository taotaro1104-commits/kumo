---
title: "LocalMind"
site_name: "LocalMind"
description: "녹음 받아쓰기, 회의록 작성, 자료 검색, AI 질문을 기기 안에서만 처리하는 Rust 제작 Android 앱입니다. 녹음과 자료는 밖으로 보내지 않습니다."
tagline: "회의도 자료도, 내 기기 안에서."
lead: "LocalMind는 녹음 받아쓰기, 회의록 작성, 내 자료 검색, AI에게 질문하기를 스마트폰 안에서만 처리하는 앱입니다. 녹음, 받아쓴 글, 자료는 기기 밖으로 나가지 않습니다."
pills: ["Android", "오프라인 사용", "Rust", "일본어 지원"]
nav_all: "앱 목록"
nav_features: "특징"
nav_how: "사용 방법"
nav_get: "받는 방법"
nav_status: "개발 상황"
stats_title: "할 수 있는 일"
stats:
  - { value: "3가지 기능", label: "회의록 작성, 자료 검색, AI에게 질문" }
  - { value: "기기 안에서", label: "녹음, 받아쓴 글, 자료는 밖으로 보내지 않습니다" }
  - { value: "3가지 모델", label: "AI 모델은 크기를 골라 쓰고, 나중에 바꿀 수 있습니다" }
stats_note: "AI 모델(약 0.5~2.1GB)은 앱에 들어 있지 않습니다. 처음 쓸 때 동의한 뒤에 내려받습니다. 작은 모델은 빠른 대신 회의록과 답변의 정확도가 떨어집니다."
shots_title: "스크린샷"
shots_note: "화면은 Android 15 에뮬레이터에서 촬영했으며, 예시 자료와 회의를 가정한 짧은 음성을 사용했습니다. 앱 화면은 현재 일본어입니다."
shots:
  - { img: "search.jpg", caption: "자료 검색(근거 표시)" }
  - { img: "groups.jpg", caption: "자료 그룹" }
  - { img: "minutes.jpg", caption: "회의록(근거 발언의 시각 표시)" }
  - { img: "ask.jpg", caption: "AI에게 질문" }
  - { img: "model.jpg", caption: "AI 모델 내려받기" }
features_title: "특징"
features:
  - { icon: "🎙️", title: "녹음하고 받아쓰기", body: "회의와 강의를 기기 안에만 녹음합니다. 녹음 전에 참석자의 동의를 확인할 수 있고, 녹음 중에는 알림이 항상 표시됩니다. 받아쓰기는 기기 안에서 처리합니다(whisper.cpp)." }
  - { icon: "📝", title: "근거가 붙는 회의록", body: "받아쓴 글에서 요약, 결정 사항, 할 일, 미해결 논점을 만듭니다. 각 항목에는 근거가 된 발언의 시각이 붙고, 근거를 확인할 수 없는 항목은 싣지 않습니다." }
  - { icon: "🔎", title: "일본어 자료 검색", body: "텍스트나 Markdown 자료를 등록하면, 표현을 바꾼 질문으로도 찾을 수 있습니다. 답이 있는 곳을 자료명·제목·줄 번호와 함께 보여 주고, 답이 없으면 '찾을 수 없습니다'라고 알려 줍니다." }
  - { icon: "🌐", title: "URL로 자료 가져오기", body: "페이지 URL을 넣으면 본문만 뽑아 자료로 만듭니다. 공개된 https 페이지만 가져오며, 내 기기나 사내 네트워크 안은 읽지 않습니다." }
  - { icon: "🗂️", title: "자료 그룹", body: "자료에 그룹을 붙여 목록을 전환할 수 있습니다. 검색도 고른 그룹으로 좁힐 수 있고, 그룹 변경이나 그룹 전체 삭제도 됩니다." }
  - { icon: "💬", title: "AI에게 질문", body: "자료를 불러오지 않고도 AI에게 질문할 수 있습니다. 기기 안에서만 답하거나, 선택했을 때만 Wikipedia를 검색해 답합니다. 검색어는 보내기 전에 보여 드리고 확인을 받습니다." }
  - { icon: "🔁", title: "AI 모델 선택과 전환", body: "Qwen2.5의 0.5B·1.5B·3B 중에서 고르거나, URL로 다른 모델(GGUF)을 지정할 수 있습니다. 내려받은 모델은 남아 있어 언제든 전환하거나 삭제할 수 있습니다." }
privacy_title: "데이터 처리"
privacy_body: "녹음, 받아쓴 글, 자료는 밖으로 보내지 않습니다. 앱이 통신하는 경우는 아래 세 가지뿐이며, 모두 사용자가 직접 선택했을 때입니다."
privacy:
  - "AI 모델 내려받기(처음 쓸 때. 접속 대상을 확인한 뒤)"
  - "AI 질문에서 'Wikipedia 검색'을 선택했을 때(AI가 만든 짧은 검색어만, 확인을 받은 뒤에 보냅니다. 질문 전체나 자료는 보내지 않습니다)"
  - "자료의 'URL로 추가'를 썼을 때(입력한 URL에 한 번만 접속합니다)"
how_title: "사용 방법"
how:
  - { title: "AI 모델 받기", body: "처음 회의록이나 질문을 쓸 때 모델 후보(또는 URL)를 골라 내려받습니다. Wi-Fi를 권장합니다." }
  - { title: "녹음하고 회의록 만들기", body: "녹음 화면에서 녹음하고 멈춘 뒤 받아쓰기를 실행합니다. 결과 화면에서 '회의록 만들기'를 누르면 몇 분 뒤 회의록이 완성되며, 공유도 할 수 있습니다." }
  - { title: "자료를 등록하고 찾기·묻기", body: "검색 화면에서 글, 파일, URL로 자료를 추가하고 질문을 입력하면 근거가 되는 부분이 나옵니다. 자료 없이 AI에게 묻고 싶을 때는 질문 화면을 사용하세요." }
download_title: "받는 방법"
download_body: "Android용 APK(앱 본체)를 Google 드라이브로 배포하고 있습니다. 소스 코드는 GitHub에 있으며, 직접 빌드할 수도 있습니다."
download_note: "APK는 직접 만든 키로 서명했기 때문에, 기기에서 \"출처를 알 수 없는 앱\"을 허용해야 합니다. 크기가 약 700MB이므로 Wi-Fi로 내려받기를 권장합니다. AI 모델은 들어 있지 않으며, 처음 쓸 때 따로 내려받습니다."
download_button: "Google 드라이브에서 내려받기"
download_url: "https://drive.google.com/drive/folders/1hfZGFiTYGIdArpSbNyXrt-hWH11323Wv?usp=drive_link"
android_file: "LocalMind-0.1.apk"
android_sha: "991d4122ff8881d79c11bad03668c3f61d3d37d86ab048d1251ccc507928d66a"
sha_label: "SHA-256(v0.1)"
cta_source: "소스 코드(GitHub)"
repo_url: "https://github.com/taotaro1104-commits/localmind"
status_title: "개발 상황"
status_body: "개발 중입니다(v0.1, 시제품). Android 15 에뮬레이터에서 녹음부터 회의록까지의 전체 흐름, 자료 검색과 가져오기, AI 질문이 동작하는 것을 확인했습니다. AI의 답변에는 오류가 있을 수 있으니 중요한 내용은 직접 확인해 주세요."
todo_title: "아직 확인하지 못했거나 지원하지 않는 것"
todo:
  - "실제 기기에서의 동작과 속도(에뮬레이터에서만 확인했으며, arm64 실제 기기의 속도는 아직 모릅니다)"
  - "작은 모델의 정확도(회의록의 분류나 질문에 대한 답이 아직 거칠 수 있습니다)"
  - "긴 회의(회의록에 쓸 수 있는 받아쓴 글은 5,000자까지입니다)"
  - "PDF 가져오기와 이미지 글자 인식(OCR)은 지원하지 않습니다"
  - "Google Play 배포(앱이 약 700MB여서 크기 제한을 넘습니다)"
footer: "© LocalMind"
---
