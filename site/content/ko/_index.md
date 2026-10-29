---
title: "Lite Simple Rust Browser"
description: "적은 메모리 사용을 최우선으로 Rust로 처음부터 만든 가볍고 단순한 웹 브라우저."
tagline: "가볍고 단순하게. Rust로 만든 웹 브라우저."
lead: "HTML·CSS 해석부터 레이아웃, 그리기, 통신, JavaScript 실행까지 자체 엔진으로 처리하므로 탭을 몇 개 열어도 가볍게 동작합니다."
nav_features: "특징"
nav_get: "받기"
cta_source: "소스 코드"
ad_label: "광고"
stats_title: "메모리 사용량 (릴리스 빌드 실측)"
stats:
  - { value: "약 22 MB", label: "빈 탭" }
  - { value: "약 65 MB", label: "위키백과 문서 1개" }
  - { value: "약 185 MB", label: "탭 10개" }
features_title: "특징"
features:
  - { title: "적은 메모리", body: "백그라운드 탭은 자동으로 동결·폐기하여 메모리 예산 안에 유지합니다." }
  - { title: "3개의 프로세스", body: "Browser, Content, Network를 분리했습니다. 페이지가 무거워도 화면이 멈추지 않으며, Content가 종료되면 자동으로 다시 만듭니다." }
  - { title: "JavaScript", body: "Boa 엔진으로 DOM, 이벤트, 타이머, fetch 등을 실행합니다." }
  - { title: "부하 수준 1~5", body: "'최경량'부터 '최대'까지 5단계. 이미지 로딩은 따로 켜고 끌 수 있습니다." }
  - { title: "비밀번호 관리자", body: "로그인할 때 저장 여부를 묻고 같은 사이트에서 자동 입력합니다. 비밀번호는 암호화되며 키는 Windows 자격 증명 관리자에 보관합니다." }
  - { title: "북마크", body: "Ctrl+D로 추가, Ctrl+B로 목록 보기. 홈페이지는 설정에서 정할 수 있습니다." }
  - { title: "방문 기록·다운로드", body: "Ctrl+H로 방문 기록, Ctrl+J로 다운로드 목록. 파일 링크는 페이지를 떠나지 않고 저장하며 진행 상황을 보여 줍니다." }
  - { title: "확대·축소", body: "Ctrl+「+」「-」 또는 Ctrl+휠로 25~500%. Ctrl+0으로 원래 크기로 돌아갑니다." }
  - { title: "일본어 입력", body: "IME로 변환한 글자를 주소창과 입력란에 넣을 수 있습니다." }
  - { title: "5개 언어 표시", body: "일본어, English, 简体中文, 한국어, Español." }
  - { title: "콘텐츠 차단", body: "필터 목록으로 광고 등의 요청을 차단합니다." }
start_title: "사용 방법"
start:
  - "실행하면 새 탭이 열립니다. 위쪽 입력란에 URL이나 검색어를 입력하세요 (Ctrl+L)."
  - "Ctrl+T 새 탭 / Ctrl+W 닫기 / Alt+← 뒤로 / Alt+Home 홈 / Ctrl+클릭 백그라운드 탭으로 열기"
  - "설정은 about:settings, 비밀번호는 about:passwords, 메모리 상태는 about:memory에서 볼 수 있습니다."
download_title: "받기"
download_button: "Google 드라이브에서 다운로드"
download_body: "Windows 11(64비트)용입니다. 설치 프로그램을 실행하거나 zip을 풀기만 하면 사용할 수 있습니다. 기록과 설정은 %LOCALAPPDATA%\\Kumo에 저장됩니다. 라이선스는 MIT 또는 Apache-2.0 중에서 선택할 수 있습니다."
release_title: "최신 버전"
release_version: "버전 0.1.0 (2026-10-29)"
release_files:
  - { name: "kumo-0.1.0-windows-x64-setup.exe", kind: "설치 프로그램", size: "8.8 MB", sha: "2f2a017fdaf6d8f85c7e70e497facdd4a00bfc2f6081c84974105158da1d1e84" }
  - { name: "kumo-0.1.0-windows-x64.zip", kind: "zip (풀기만 하면 사용 가능)", size: "10.4 MB", sha: "d78c7542a9a61225636428c30e1b2fc8d30df193cc19554963bbc030317bdba8" }
sha_note: "내려받은 파일의 SHA-256이 아래 값과 같은지 확인할 수 있습니다."
changes_title: "최근 업데이트"
changes:
  - "페이지 확대·축소 추가 (Ctrl+「+」「-」, Ctrl+휠, Ctrl+0)"
  - "일본어 입력(IME) 지원, 변환 중인 글자는 주소창에 표시"
  - "방문 기록(Ctrl+H)과 다운로드 목록(Ctrl+J) 추가. 파일 링크는 페이지를 떠나지 않고 저장"
  - "앱 이름을 Lite Simple Rust Browser로 바꾸고 고유 아이콘 추가"
  - "비밀번호 관리자와 5개 언어 표시 추가"
limits_title: "현재의 제한"
limits_body: "개발 중이므로 모든 사이트가 완벽하게 표시되지는 않습니다. ES 모듈, iframe, canvas, GPU 렌더링, 인라인 SVG 등은 아직 지원하지 않습니다."
shop_title: "추천"
shop_lead: "컴퓨터, 주변기기, 일상 용품은 아래 일본 쇼핑몰에서 찾아볼 수 있습니다."
rakuten_label: "라쿠텐 이치바에서 찾기"
amazon_label: "Amazon.co.jp에서 찾기"
shop_disclosure: "이 사이트는 광고(Google AdSense)와 제휴 프로그램(라쿠텐 어필리에이트, Amazon 어소시에이트)을 이용합니다. Amazon 어소시에이트로서 이 사이트는 적격 구매를 통해 수입을 얻습니다."
---
