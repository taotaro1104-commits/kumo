{
  "description": "Rust EQ 是一款 Android 本地音乐播放器，提供16种预设、10／31段均衡器、虚拟低音和空间音效，让你轻松调整喜欢的声音。",
  "tagline": "熟悉的歌，\n喜欢的声音。",
  "lead": "先选一个预设，再加一点低音，或让人声更清晰。Rust EQ 让你一边聆听 Android 设备中的音乐，一边找到适合自己的声音平衡。",
  "pills": [
    "Android 10 及以上",
    "离线播放",
    "开发中 v0.4.5",
    "日本語 / English"
  ],
  "nav_all": "所有应用",
  "nav_features": "功能",
  "nav_shots": "界面",
  "nav_get": "下载",
  "download_button": "从 Google 云端硬盘获取",
  "source_button": "查看源代码",
  "hero_caption": "用均衡器曲线，直观看到声音的平衡。",
  "features_title": "轻松上手，也能细细调整。",
  "features": [
    {
      "title": "16种预设，随心选择",
      "body": "从平直、人声、丰富低音等预设开始。通过小曲线预览，比较不同的声音风格。"
    },
    {
      "title": "低音、中音、高音，自由调整",
      "body": "通过10／31段参数均衡器调整频率、增益和Q值，还可使用虚拟低音、压缩器和限制器。"
    },
    {
      "title": "探索宽度与空间感",
      "body": "调整立体声宽度和交叉馈送，使用 HRTF／SOFA 和 IR 混响体验耳机的空间音效。"
    },
    {
      "title": "保存喜欢的声音",
      "body": "保存自定义预设，以 JSON 导入或导出，也支持导入 AutoEQ 文本和 CSV 曲线进行拟合。"
    }
  ],
  "shots_title": "三个界面，清楚好找。",
  "shots_note": "功能按“声音”“空间”“工具”分组。截图来自 Android 模拟器，应用界面为日语。点击图片可查看原图。",
  "shots": [
    {
      "img": "sound.png",
      "caption": "声音",
      "body": "预设与均衡器曲线，一屏掌握。"
    },
    {
      "img": "space.png",
      "caption": "空间",
      "body": "参考扬声器示意图，调整立体声宽度。"
    },
    {
      "img": "tools.png",
      "caption": "工具",
      "body": "集中保存、导入声音设置。"
    },
    {
      "img": "presets.png",
      "caption": "预设列表",
      "body": "比较曲线，选一个喜欢的起点。"
    }
  ],
  "how_title": "从喜欢的歌开始。",
  "how": [
    {
      "title": "打开音乐",
      "body": "通过 Android 文件选择器选择歌曲，也可打开文件夹，将歌曲加入播放队列。"
    },
    {
      "title": "找到喜欢的平衡",
      "body": "选择预设后播放。切换均衡器开关对比效果，边听边做小幅调整。"
    },
    {
      "title": "保存设置",
      "body": "在“工具”中保存喜欢的设置，也可通过通知控制播放、暂停和切歌。"
    }
  ],
  "download_title": "试试 Rust EQ",
  "download_body": "从Google Drive分发文件夹获取Android签名APK。0.4.5的文件名为rust-eq-0.4.5-release.apk。",
  "download_note": "需要 Android 10 及以上，支持 arm64-v8a 和 x86_64。保存 APK 后，按照设备提示安装。界面支持英语和日语，随设备语言偏好切换；Android 13 及以上也可在系统的应用语言设置中选择。 下载前请核对文件夹内的文件名和版本。",
  "details_title": "开始之前，了解这些",
  "details": [
    {
      "title": "支持的音乐文件",
      "body": "支持单声道和立体声的 MP3、WAV、FLAC、Ogg Vorbis、AAC-LC／M4A、ALAC 和 AIFF 音频。不支持受 DRM 保护的音乐。"
    },
    {
      "title": "可以为其他应用使用均衡器吗？",
      "body": "选择并保存目标后轻点Connect。不通知音频会话的播放器，可尝试可选的Enhanced detection和额外ADB设置，并查看实际连接状态。设备EQ仅适用于兼容音频会话，无法强制将所有应用的声音送入Rust DSP。虚拟低音、动态处理、HRTF和IR用于应用内播放。"
    },
    {
      "title": "为什么没有显示曲目信息？",
      "body": "选择YouTube Music并允许Android通知访问。它不同于普通通知权限和EQ扩展检测的可选DUMP设置。Android 13及以上若阻止设置，请检查应用信息 → 菜单 → 允许受限设置。播放器必须公开媒体信息才能显示。"
    },
    {
      "title": "音乐与设置如何处理？",
      "body": "应用本体不请求网络或麦克风权限。选定的音乐在设备本地处理，通过 Android 文件选择器打开文件。"
    },
    {
      "title": "开发与测试进度",
      "body": "v0.4.5通过Android 13上的42项联动测试、15项单元测试和35项Rust测试。已验证同签名升级、设置保留和进程结束后的实时曲目信息恢复。尚未在实机上验证当前版本的YouTube Music。"
    }
  ],
  "footer": "让声音更合心意。Rust EQ",
  "listing_tagline": "找到喜欢的声音。",
  "listing_body": "用16种预设和10／31段均衡器调整本地音乐，透过清晰直观的界面探索虚拟低音与空间音效。",
  "listing_tags": [
    "Android 10 及以上",
    "离线播放",
    "Rust DSP"
  ],
  "listing_cta": "了解 Rust EQ",
  "title": "Rust EQ",
  "site_name": "Rust EQ",
  "type": "rusteq",
  "download_url": "https://drive.google.com/drive/folders/1hImCSXhyR_w83_6F2caRq50dvMjMIgfF?usp=drive_link",
  "repo_url": "https://github.com/taotaro1104-commits/RustAndroidequalizer",
  "og_image": "rusteq/og-v2.png",
  "og_image_width": 1731,
  "og_image_height": 909,
  "og_image_alt": "Rust EQ，Android音乐播放器与均衡器，提供16种预设、10／31段EQ和离线播放。",
  "external_title": "其他播放器，也能沿用你的EQ设置。",
  "external_lead": "选择并保存兼容的应用。尚未连接时，轻点“连接”即可打开播放器。",
  "external_steps": [
    {
      "title": "选择目标应用",
      "body": "按应用名称搜索并勾选需要使用EQ的播放器，支持多选。"
    },
    {
      "title": "保存选择",
      "body": "保存目标应用和监听设置，下次继续使用。"
    },
    {
      "title": "轻点连接",
      "body": "轻点“连接”后，在播放器中重新播放歌曲。已连接时可用“重新连接”重试。"
    }
  ],
  "external_note": "仅对公开音频会话的兼容应用应用设备EQ，可启用重启后恢复监听。无法强制将所有应用的声音送入Rust DSP。截图中的新设置为英文界面。",
  "external_shots": [
    {
      "img": "target-picker-en.png",
      "caption": "选择应用并保存"
    },
    {
      "img": "target-connected-en.png",
      "caption": "查看状态并重新连接"
    }
  ],
  "playback_title": "YouTube Music正在播放什么，一目了然。",
  "playback_lead": "在Sound和Playback details中查看所选播放器的曲名、歌手和播放状态。",
  "playback_steps": [
    {
      "title": "选择YouTube Music",
      "body": "在Tools → EQ for other apps中选择并保存YouTube Music。它与YouTube是两个独立应用。"
    },
    {
      "title": "允许通知访问",
      "body": "打开Playback details → Set up playback information access，在Android设置中允许通知访问。"
    },
    {
      "title": "播放歌曲后返回",
      "body": "在YouTube Music中播放歌曲，再返回Rust EQ。必要时轻点Refresh playback information。"
    }
  ],
  "playback_note": "曲目信息与EQ连接分别显示。不会分析通知正文或保存收听历史。手机EQ不适用于投屏设备上的音频。"
}
