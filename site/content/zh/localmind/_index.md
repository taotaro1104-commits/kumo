---
title: "LocalMind"
site_name: "LocalMind"
description: "一款用 Rust 编写的 Android 应用，在设备本地完成录音转写、会议纪要、资料搜索和 AI 问答。录音和资料都不会发送到设备之外。"
tagline: "会议和资料，都留在你的设备里。"
lead: "LocalMind 在手机内部完成录音转写、生成会议纪要、搜索自己的资料以及向 AI 提问。录音、转写文字和资料都不会离开设备。"
pills: ["Android", "可离线使用", "Rust", "支持日语"]
nav_all: "全部应用"
nav_features: "特点"
nav_how: "使用方法"
nav_get: "获取方式"
nav_status: "开发进展"
stats_title: "能做什么"
stats:
  - { value: "3 项功能", label: "会议纪要、资料搜索、向 AI 提问" }
  - { value: "在设备内", label: "录音、转写文字和资料都不会发送到外部" }
  - { value: "3 种模型", label: "AI 模型可按大小选择，之后还能切换" }
stats_note: "AI 模型（约 0.5～2.1 GB）不随应用附带。首次使用时，经你同意后下载。较小的模型速度更快，但纪要和回答的准确度会下降。"
shots_title: "截图"
shots_note: "截图来自 Android 15 模拟器，使用的是示例资料和一段模拟会议的短音频。目前应用界面为日语。"
shots:
  - { img: "search.jpg", caption: "资料搜索（附依据）" }
  - { img: "groups.jpg", caption: "资料分组" }
  - { img: "minutes.jpg", caption: "会议纪要（附依据发言的时间）" }
  - { img: "ask.jpg", caption: "向 AI 提问" }
  - { img: "model.jpg", caption: "下载 AI 模型" }
features_title: "特点"
features:
  - { icon: "🎙️", title: "录音并转写", body: "会议和课程只录在设备本地。录音前可以确认参与者已同意，录音期间始终显示通知。转写在设备本地进行（whisper.cpp）。" }
  - { icon: "📝", title: "附依据的会议纪要", body: "根据转写文字生成摘要、决定事项、待办事项和未解决的问题。每一项都标有所依据发言的时间，无法确认依据的项目不会列入。" }
  - { icon: "🔎", title: "日语资料搜索", body: "登记文本或 Markdown 资料后，换一种说法提问也能找到。会标出答案所在的资料名、标题和行号；没有答案时会回答“未找到”。" }
  - { icon: "🌐", title: "从 URL 导入资料", body: "输入网页的 URL，只提取正文作为资料。只获取公开的 https 网页，不会读取你自己的设备或内部网络。" }
  - { icon: "🗂️", title: "资料分组", body: "可以给资料分组并切换列表。搜索也能限定在所选分组内。可以更换分组，也可以整组一并删除。" }
  - { icon: "💬", title: "向 AI 提问", body: "不加载任何资料也能向 AI 提问。可以只在设备内回答，也可以仅在你选择时先搜索 Wikipedia。搜索词会在发送前展示给你确认。" }
  - { icon: "🔁", title: "选择并切换 AI 模型", body: "可从 Qwen2.5 的 0.5B、1.5B、3B 中选择，也可以用 URL 指定其他模型（GGUF）。下载过的模型会保留，随时可以切换或删除。" }
privacy_title: "数据如何处理"
privacy_body: "录音、转写文字和资料都不会发送到外部。应用只在以下三种情况下联网，而且都是你自己选择的操作。"
privacy:
  - "下载 AI 模型（首次使用时，确认来源后下载）"
  - "向 AI 提问时选择“搜索 Wikipedia”（只发送 AI 生成的简短搜索词，并且在发送前由你确认。不会发送完整的问题或资料）"
  - "使用资料的“从 URL 添加”（只连接一次你输入的 URL）"
how_title: "使用方法"
how:
  - { title: "获取 AI 模型", body: "首次生成纪要或提问时，选择模型（或输入 URL）并下载。建议使用 Wi-Fi。" }
  - { title: "录音并生成纪要", body: "在录音界面录音、停止，然后执行转写。在结果界面点击“生成会议纪要”，几分钟后纪要就完成了，还可以分享。" }
  - { title: "添加资料，搜索或提问", body: "在搜索界面，通过文字、文件或 URL 添加资料，输入问题就会显示作为依据的段落。想不依赖资料直接问 AI 时，请使用提问界面。" }
download_title: "获取方式"
download_body: "Android 用的 APK（应用本体）通过 Google 云端硬盘分发。源代码在 GitHub 上，也可以自行构建。"
download_note: "APK 使用我们自己生成的密钥签名，所以设备会要求你允许安装“未知来源的应用”。文件约 700 MB，建议使用 Wi-Fi 下载。AI 模型不包含在内，首次使用时需要另行下载。"
download_button: "从 Google 云端硬盘下载"
download_url: "https://drive.google.com/drive/folders/1hfZGFiTYGIdArpSbNyXrt-hWH11323Wv?usp=drive_link"
android_file: "LocalMind-0.1.apk"
android_sha: "991d4122ff8881d79c11bad03668c3f61d3d37d86ab048d1251ccc507928d66a"
sha_label: "SHA-256（v0.1）"
cta_source: "源代码（GitHub）"
repo_url: "https://github.com/taotaro1104-commits/localmind"
status_title: "开发进展"
status_body: "开发中（v0.1，原型版）。已在 Android 15 模拟器上确认：从录音到纪要的完整流程、资料的搜索与导入、向 AI 提问都能运行。AI 的回答可能有误，重要内容请自行核实。"
todo_title: "尚未确认或暂不支持"
todo:
  - "真机上的运行情况和速度（仅在模拟器上确认过，arm64 真机的速度尚未确认）"
  - "小模型的准确度（纪要的分类和问题的回答可能仍比较粗略）"
  - "较长的会议（用于生成纪要的转写文字最多 5,000 字）"
  - "暂不支持导入 PDF 和从图片识别文字（OCR）"
  - "在 Google Play 上发布（应用约 700 MB，超过大小上限）"
footer: "© LocalMind"
---
