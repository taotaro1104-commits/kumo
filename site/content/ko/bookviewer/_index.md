---
title: "RustHighBookViewer"
site_name: "RustHighBookViewer"
description: "Rust로 만든 만화·이미지 뷰어. ZIP/CBZ, 7z, RAR, EPUB, 이미지 폴더를 오른쪽에서 왼쪽으로 넘기는 펼침 보기 등으로 읽을 수 있습니다. Android와 Windows를 지원합니다."
tagline: "열면 바로 읽힌다."
lead: "RustHighBookViewer는 만화나 이미지가 들어 있는 압축 파일과 폴더를 열어 읽는 뷰어입니다. 오른쪽에서 왼쪽으로 넘기는 펼침 보기, 확대, 색 조정, 읽던 위치 저장을 지원하며 Android와 Windows에서 동작합니다."
pills: ["Android", "Windows", "ZIP / CBZ / 7z / RAR / EPUB", "오른쪽 제본 지원"]
nav_all: "앱 목록"
nav_features: "특징"
nav_how: "사용법"
nav_get: "다운로드"
nav_status: "개발 상황"
stats_title: "지원하는 것"
stats:
  - { value: "5종류", label: "ZIP/CBZ, 7z, RAR, EPUB, 이미지 폴더를 열 수 있습니다" }
  - { value: "4가지 보기", label: "한 페이지, 펼침, 자동 펼침, 세로 스크롤" }
  - { value: "2개 환경", label: "Android 8.0 이상, 그리고 Windows" }
stats_note: "RAR은 4.x / 5.x(암호 없음)를 지원합니다. 7z와 RAR은 열 때 기기의 저장 공간에 먼저 풀어 둡니다. EPUB은 한 페이지가 이미지 한 장인 책(만화, 사진집)을 지원합니다."
features_title: "특징"
features:
  - { icon: "📖", title: "오른쪽 제본도 왼쪽 제본도", body: "제본 방향을 바꿀 수 있고, 펼침 순서도 함께 바뀝니다. 표지는 혼자 보여 주고, 가로로 긴 페이지는 한 장으로 표시합니다." }
  - { icon: "🔍", title: "확대와 돋보기", body: "핀치나 더블 탭으로 확대하고 이동할 수 있습니다. Android 버전에는 손가락으로 훑은 곳을 2배로 키워 보여 주는 돋보기도 있습니다." }
  - { icon: "🗂️", title: "페이지 목록", body: "책 전체의 썸네일에서 읽고 싶은 페이지로 바로 이동합니다(Android 버전)." }
  - { icon: "🎨", title: "색 조정", body: "밝기, 대비, 감마, 흑백, 세피아, 반전. 설정은 책마다 기억합니다(Android 버전)." }
  - { icon: "⏱️", title: "자동 넘김", body: "3·5·10·20초마다 자동으로 페이지를 넘기고, 마지막 페이지에서 멈춥니다(Android 버전)." }
  - { icon: "📚", title: "책장과 이어 읽기", body: "기록, 즐겨찾기, 제목 검색이 있는 책장. 닫아도 읽던 위치를 저장하므로 이어서 읽을 수 있습니다." }
  - { icon: "📁", title: "폴더도 열기", body: "폴더를 고르면 각 폴더의 이미지는 한 권으로, EPUB·ZIP·7z·RAR는 파일 하나당 한 권으로 묶어 하위 폴더까지 한꺼번에 책장에 추가합니다. Android에서는 “모든 파일 액세스”를 허용하면 Download 같은 폴더도 바로 고를 수 있습니다." }
  - { icon: "🛡️", title: "안전 고려", body: "풀 수 있는 크기와 파일 수에 상한이 있고, 남은 공간이 적으면 도중에 멈춥니다. 압축 파일 안의 ../ 같은 경로는 무효로 처리합니다." }
  - { icon: "🔢", title: "페이지 지정과 책갈피", body: "페이지 번호를 입력해 바로 이동하거나, 책갈피 목록에서 이동할 수 있습니다. 한 번 눌러 책갈피를 추가·해제합니다(Android 버전)." }
  - { icon: "⚙️", title: "설정", body: "기본 보기 방식, 제본 방향, 맞춤, 볼륨 키 넘김, 화면 켜 두기, 책장 정렬과 이미지 캐시가 쓸 메모리를 정할 수 있습니다(Android 버전)." }
  - { icon: "📘", title: "EPUB 만화", body: "한 페이지가 이미지 한 장으로 된 EPUB을 읽는 순서대로 엽니다. 글 위주의 EPUB은 지원하지 않습니다." }
  - { icon: "⚡", title: "두 번째부터 빠르게", body: "페이지 크기를 기억해 두므로, 폴더와 압축 파일을 다음에 열 때 더 빠릅니다." }
how_title: "사용법"
how:
  - { title: "열기", body: "Android에서는 책장의 '파일' 또는 '폴더'에서 고릅니다. Windows에서는 exe에 파일이나 폴더를 끌어다 놓습니다." }
  - { title: "읽기", body: "화면의 왼쪽이나 오른쪽을 탭해서 페이지를 넘깁니다. 가운데를 탭하면 메뉴가 나타나 보기 방식과 제본 방향을 바꿀 수 있습니다." }
  - { title: "이어 읽기", body: "닫으면 읽던 위치를 저장합니다. 책장에서 고르면 이어서 열립니다." }
download_title: "다운로드"
download_button: "Google 드라이브에서 다운로드"
download_body: "Android용 APK와 Windows용 exe를 Google 드라이브에서 배포합니다. 설치 프로그램은 없으며, exe는 실행만 하면 됩니다."
download_note: "APK는 직접 만든 키로 서명했기 때문에 기기에서 '출처를 알 수 없는 앱'을 허용해야 합니다. exe는 서명하지 않아 Windows SmartScreen이 경고를 표시할 수 있습니다."
cta_source: "소스 코드"
sha_label: "SHA-256(v0.1.3)"
release_label: "변경 내역·소스 코드(GitHub)"
download_url: "https://drive.google.com/drive/folders/15qKnVY5FglIUeVdFlwyLt3eVNU_XtltK?usp=sharing"
repo_url: "https://github.com/taotaro1104-commits/RustHighBookViewer"
release_url: "https://github.com/taotaro1104-commits/RustHighBookViewer/releases"
android_file: "RustHighBookViewer-0.1.3-android.apk"
windows_file: "RustHighBookViewer-desktop.exe"
android_sha: "2bef5262f6cbb348b7961bee0426102f33318eae24f88c17bb6ad4bf15d6d5fc"
windows_sha: "5211618dec0cff0fc5f1179d5a7de8f4f4556fc6baadc59ac90c4b095672854a"
status_title: "개발 상황"
status_body: "개발 중입니다(v0.1.3, 프리릴리스). Android는 Android 15 에뮬레이터에서 7z, RAR, EPUB, 폴더 일괄 가져오기가 동작함을 확인했습니다. 소스 코드는 GitHub에 공개되어 있습니다."
todo_title: "아직 확인하지 못한 것·지원하지 않는 것"
todo:
  - "Android 실제 기기에서의 동작(에뮬레이터에서만 확인)"
  - "Windows 버전의 화면 모습과 조작(실행되어 계속 동작하는 것까지만 확인)"
  - "암호가 걸린 압축 파일과, 여러 파일로 나뉜 RAR(지원하지 않음)"
footer: "© RustHighBookViewer — MIT License"
---
