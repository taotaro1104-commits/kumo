{
  "description": "Rust EQ는 16가지 프리셋과 10/31밴드 EQ, 가상 저음, 공간 효과로 기기에 저장된 음악을 조절하는 Android 앱입니다.",
  "tagline": "익숙한 노래를,\n내가 좋아하는 소리로.",
  "lead": "프리셋 하나로 시작해 보세요. 저음을 조금 더하거나 보컬을 또렷하게. Rust EQ는 Android 기기에 저장된 음악을 들으며 나에게 맞는 소리의 균형을 찾는 앱입니다.",
  "pills": [
    "Android 10 이상",
    "오프라인 재생",
    "개발 중 v0.4.8",
    "日本語 / English"
  ],
  "nav_all": "모든 앱",
  "nav_features": "기능",
  "nav_shots": "화면",
  "nav_get": "다운로드",
  "download_button": "Google 드라이브에서 받기",
  "source_button": "소스 코드 보기",
  "hero_caption": "EQ 곡선으로 소리의 균형을 확인하세요.",
  "features_title": "가볍게 시작하고, 세밀하게 조절하세요.",
  "features": [
    {
      "title": "기분에 맞게 고르는 16가지 프리셋",
      "body": "플랫, 보컬, 풍부한 저음 등에서 시작하세요. 작은 곡선 미리보기로 록, 클래식 등 31밴드용 프리셋도 추가되었습니다. 소리의 성격을 비교할 수 있습니다."
    },
    {
      "title": "저음부터 고음까지 자유롭게",
      "body": "10/31밴드 파라메트릭 EQ로 주파수, 게인, Q를 조절합니다. 가상 저음은 최대 200%까지 강화할 수 있으며 컴프레서, 리미터, 에코(양·간격·반복), 리버브, 다이내믹 저음 EQ, 트루 피크 리미터도 사용할 수 있습니다."
    },
    {
      "title": "넓이와 공간감까지",
      "body": "스테레오 폭과 크로스피드를 조절하고, HRTF/SOFA와 IR 리버브로 헤드폰 공간 효과를 시험해 보세요."
    },
    {
      "title": "찾아낸 소리를 저장하기",
      "body": "직접 만든 프리셋을 저장하고 JSON으로 내보내거나 가져옵니다. AutoEQ 텍스트와 CSV 곡선을 가져와 맞추는 기능도 지원합니다."
    }
  ],
  "shots_title": "세 가지 화면으로 쉽게 조절하세요.",
  "shots_note": "기능을 소리, 공간, 도구로 나눴습니다. Android 에뮬레이터에서 촬영한 일본어 UI입니다. 이미지를 누르면 원본 크기로 열립니다.",
  "shots": [
    {
      "img": "sound.png",
      "caption": "소리",
      "body": "프리셋과 EQ 곡선을 한 화면에서."
    },
    {
      "img": "space.png",
      "caption": "공간",
      "body": "스피커 그림을 보며 스테레오 폭을 조절."
    },
    {
      "img": "tools.png",
      "caption": "도구",
      "body": "설정 저장과 가져오기를 한곳에서."
    },
    {
      "img": "presets.png",
      "caption": "프리셋 목록",
      "body": "곡선을 비교하며 마음에 드는 소리를 선택."
    }
  ],
  "how_title": "좋아하는 노래로 시작하세요.",
  "how": [
    {
      "title": "음악 열기",
      "body": "Android 파일 선택 화면에서 곡을 고르세요. 폴더를 열어 곡을 재생 대기열에 추가할 수도 있습니다."
    },
    {
      "title": "취향에 맞게 조절",
      "body": "프리셋을 선택하고 재생하세요. EQ를 켜고 끄며 비교하고, 들으면서 조금씩 조절합니다."
    },
    {
      "title": "설정 저장",
      "body": "마음에 드는 설정을 도구에서 저장하세요. 알림에서도 재생, 일시 정지, 곡 이동을 조작할 수 있습니다."
    }
  ],
  "download_title": "Rust EQ 사용해 보기",
  "download_body": "Google Drive 배포 폴더에서 Android 서명 APK를 받을 수 있습니다. 0.4.8 파일명은 rust-eq-0.4.8-release.apk입니다.",
  "download_note": "Android 10 이상 / arm64-v8a, x86_64 지원. APK를 저장한 뒤 기기의 안내에 따라 설치하세요. 일본어와 영어 UI를 지원하며 기기의 언어 설정을 따릅니다. Android 13 이상에서는 앱별 언어 설정에서도 선택할 수 있습니다. 다운로드 전 폴더의 파일명과 버전을 확인하세요.",
  "details_title": "시작 전에 알아두세요",
  "details": [
    {
      "title": "지원하는 음악 파일",
      "body": "모노 및 스테레오 MP3, WAV, FLAC, Ogg Vorbis, AAC-LC/M4A, ALAC, AIFF 오디오를 재생합니다. DRM으로 보호된 음악은 지원하지 않습니다."
    },
    {
      "title": "다른 앱에도 EQ를 적용할 수 있나요?",
      "body": "대상을 선택해 저장하고 Connect를 누르세요. 오디오 세션을 알리지 않는 앱은 선택적 Enhanced detection과 추가 ADB 설정으로 연결될 수 있습니다. 실제 연결 상태를 확인하세요. 기기 EQ는 호환 세션용이며 모든 앱의 출력을 Rust DSP로 강제 전달하지 않습니다. 가상 저음, 다이내믹스, HRTF, IR은 앱 내 재생용입니다."
    },
    {
      "title": "곡 정보가 표시되지 않나요?",
      "body": "YouTube Music을 선택하고 Android의 알림 접근을 허용하세요. 일반 알림 권한 및 EQ 확장 감지의 선택적 DUMP 설정과는 별개입니다. Android 13 이상에서 차단되면 앱 정보 → 메뉴 → 제한된 설정 허용을 확인하세요. 플레이어가 미디어 정보를 공개해야 표시할 수 있습니다."
    },
    {
      "title": "음악과 설정의 처리",
      "body": "앱 자체는 인터넷 권한을 요청하지 않습니다. 마이크 권한은 ‘스피커와 방 보정’을 시작할 때만 요청하며, 음성은 저장하거나 전송하지 않습니다. 선택한 음악은 기기 안에서 처리하고 Android 파일 선택 화면으로 파일을 엽니다."
    },
    {
      "title": "개발 및 검증 상황",
      "body": "v0.4.8은 비교 청취·음색 맞춤 도구와 리버브 등을 추가했습니다. Rust 테스트 52개, Android 15 에뮬레이터의 Android 테스트 54개, Lint, APK 서명을 확인했지만 실제 기기 테스트와 청감 확인은 하지 않았습니다. v0.4.5는 Android 13 연동 테스트 42개, 단위 테스트 15개, Rust 테스트 35개를 통과했습니다. 동일 서명 업데이트, 설정 유지, 프로세스 종료 후 곡 정보 재취득을 확인했습니다. 현재 YouTube Music의 실제 기기 동작은 미검증입니다."
    }
  ],
  "footer": "내 취향의 소리. Rust EQ",
  "listing_tagline": "소리를 내 취향대로.",
  "listing_body": "16가지 프리셋과 10/31밴드 EQ로 기기의 음악을 조절하세요. 보기 쉬운 화면에서 가상 저음과 공간 효과도 경험할 수 있습니다.",
  "listing_tags": [
    "Android 10 이상",
    "오프라인 재생",
    "Rust DSP"
  ],
  "listing_cta": "Rust EQ 알아보기",
  "title": "Rust EQ",
  "site_name": "Rust EQ",
  "type": "rusteq",
  "download_url": "https://drive.google.com/drive/folders/1hImCSXhyR_w83_6F2caRq50dvMjMIgfF?usp=drive_link",
  "repo_url": "https://github.com/taotaro1104-commits/RustAndroidequalizer",
  "og_image": "rusteq/og-v2.png",
  "og_image_width": 1731,
  "og_image_height": 909,
  "og_image_alt": "Rust EQ. 16가지 프리셋, 10/31밴드 EQ와 오프라인 재생을 지원하는 Android 음악 앱.",
  "external_title": "다른 플레이어에서도, 익숙한 EQ로.",
  "external_lead": "호환 앱을 선택해 저장하세요. 연결되지 않은 앱은 연결 버튼을 눌러 열 수 있습니다.",
  "external_steps": [
    {
      "title": "앱 선택",
      "body": "이름으로 검색해 EQ를 적용할 플레이어를 선택하세요. 여러 앱을 고를 수 있습니다."
    },
    {
      "title": "선택 저장",
      "body": "선택한 앱과 대기 설정을 저장해 다음에도 그대로 사용하세요."
    },
    {
      "title": "눌러서 연결",
      "body": "Connect를 누르고 플레이어에서 곡을 다시 재생하세요. 연결 중에는 Reconnect로 재시도할 수 있습니다."
    }
  ],
  "external_note": "오디오 세션을 공유하는 호환 앱에 기기 EQ를 적용합니다. 재시작 후 대기 복원을 설정할 수 있습니다. 모든 앱의 소리를 Rust DSP로 강제 전달할 수는 없습니다. 새 설정 화면은 영어로 표시됩니다.",
  "external_shots": [
    {
      "img": "target-picker-en.png",
      "caption": "앱 선택과 저장"
    },
    {
      "img": "target-connected-en.png",
      "caption": "연결 상태 확인과 재연결"
    }
  ],
  "playback_title": "YouTube Music의 재생 정보도 한눈에.",
  "playback_lead": "선택한 플레이어의 곡명, 아티스트, 재생 상태를 Sound와 Playback details에서 확인하세요.",
  "playback_steps": [
    {
      "title": "YouTube Music 선택",
      "body": "Tools → EQ for other apps에서 YouTube Music을 선택하고 저장하세요. YouTube와는 별도 앱입니다."
    },
    {
      "title": "알림 접근 허용",
      "body": "Playback details → Set up playback information access를 열고 Android 설정에서 알림 접근을 허용하세요."
    },
    {
      "title": "곡을 재생하고 돌아오기",
      "body": "YouTube Music에서 재생한 뒤 Rust EQ로 돌아오세요. 필요하면 Refresh playback information을 누르세요."
    }
  ],
  "playback_note": "곡 정보와 EQ 연결 상태는 따로 표시됩니다. 알림 본문을 분석하거나 청취 기록을 저장하지 않습니다. 캐스트 기기의 소리에는 휴대전화 EQ가 적용되지 않습니다."
}
