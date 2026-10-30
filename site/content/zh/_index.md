---
title: "Lite Simple Rust Browser"
description: "一款以低内存占用为首要目标、用 Rust 从零编写的轻量简洁的网页浏览器。"
tagline: "轻巧、简洁。用 Rust 打造的网页浏览器。"
lead: "从 HTML 和 CSS 的解析，到布局、绘制、网络通信和 JavaScript 的执行，全部由自研引擎完成，无论打开多少标签页都保持轻快。"
nav_features: "特点"
nav_get: "获取"
cta_source: "源代码"
ad_label: "广告"
stats_title: "内存占用（发布版实测）"
stats:
  - { mb: 22, value: "约 22 MB", label: "空白标签页" }
  - { mb: 65, value: "约 65 MB", label: "一篇维基百科文章" }
  - { mb: 185, value: "约 185 MB", label: "10 个标签页" }
features_title: "特点"
features:
  - { icon: "🍃", title: "省内存", body: "后台标签页会自动冻结或丢弃，使内存保持在预算之内。" }
  - { icon: "🔀", title: "三个进程", body: "Browser、Content、Network 相互独立。页面再重窗口也不会卡住，Content 意外退出时会自动重建。" }
  - { icon: "⚡", title: "JavaScript", body: "基于 Boa 引擎运行 DOM、事件、定时器和 fetch 等。" }
  - { icon: "🎚️", title: "负载级别 1–5", body: "从“最轻”到“最大”共 5 档。图片加载可单独开关。" }
  - { icon: "🔑", title: "密码管理器", body: "登录时询问是否保存，并在同一网站自动填入。密码经过加密，密钥保存在 Windows 凭据管理器中。" }
  - { icon: "⭐", title: "书签", body: "Ctrl+D 添加，Ctrl+B 查看列表。主页可在设置中指定。" }
  - { icon: "🕘", title: "历史记录与下载", body: "Ctrl+H 查看历史，Ctrl+J 查看下载列表。点击文件链接不离开当前页面即可保存，并显示进度。" }
  - { icon: "🔍", title: "页面缩放", body: "Ctrl+加号、Ctrl+减号或 Ctrl+滚轮，可在 25% 到 500% 之间缩放。Ctrl+0 恢复原大小。" }
  - { icon: "あ", title: "日语输入", body: "用输入法（IME）转换的文字可输入地址栏和页面输入框。" }
  - { icon: "🌏", title: "5 种显示语言", body: "日语、English、简体中文、한국어、Español。" }
  - { icon: "🛡️", title: "内容拦截", body: "通过过滤列表拦截广告等请求。" }
start_title: "使用方法"
start_lead: "启动后会打开新标签页。在顶部输入框中输入网址或搜索词即可。"
keys:
  - { k: "Ctrl L", d: "转到地址栏" }
  - { k: "Ctrl T", d: "新标签页" }
  - { k: "Ctrl W", d: "关闭标签页" }
  - { k: "Ctrl D", d: "为此页添加书签" }
  - { k: "Ctrl H", d: "历史记录" }
  - { k: "Ctrl J", d: "下载列表" }
  - { k: "Ctrl +/-", d: "放大或缩小" }
start_note: "设置在 about:settings，密码在 about:passwords，内存状况在 about:memory。"
download_title: "获取"
download_button: "从 Google 云端硬盘下载"
recommended: "推荐"
sha_summary: "查看校验值（SHA-256）"
download_body: "适用于 Windows 11（64 位）。运行安装程序，或直接解压 zip 即可使用。历史记录和设置保存在 %LOCALAPPDATA%\\Kumo。可在 MIT 或 Apache-2.0 许可证中任选其一。 适用于 Android 8.0 及以上（需允许安装未知来源的应用）。"
release_title: "最新版本"
release_version: "版本 0.1.0（2026-10-29）"
release_files:
  - { name: "kumo-0.1.0-windows-x64-setup.exe", kind: "安装程序", size: "8.8 MB", sha: "2f2a017fdaf6d8f85c7e70e497facdd4a00bfc2f6081c84974105158da1d1e84" }
  - { name: "kumo-0.1.0-windows-x64.zip", kind: "zip（解压即用）", size: "10.4 MB", sha: "d78c7542a9a61225636428c30e1b2fc8d30df193cc19554963bbc030317bdba8" }
  - { name: "kumo-0.1.0-android-arm64.apk", kind: "Android 应用（arm64）", size: "12.1 MB", sha: "68bb339dc891b375dd2fcc8f3ae131e8fa8a32a42c1517e2316f92eda4b29478" }
sha_note: "可将下载文件的 SHA-256 与下列数值对比校验。"
changes_title: "最近更新"
changes:
  - "新增 Android 版（触摸滚动、点按、双指缩放、屏幕键盘、输入法组合输入、密码保存）"
  - "新增页面缩放（Ctrl+加号、Ctrl+减号、Ctrl+滚轮、Ctrl+0）"
  - "支持日语输入（IME），转换中的文字显示在地址栏"
  - "新增历史记录（Ctrl+H）和下载列表（Ctrl+J），点击文件链接无需离开页面即可保存"
  - "更名为 Lite Simple Rust Browser，并新增原创图标"
  - "新增密码管理器和 5 种显示语言"
limits_title: "当前限制"
limits_body: "软件仍在开发中，并非所有网站都能完美显示。尚不支持 ES 模块、iframe、canvas、GPU 渲染和内联 SVG 等。 Android 版为试验性质。输入法组合输入取决于键盘应用，页面输入框内不显示组合中的文字。"
shop_title: "推荐"
shop_lead: "电脑、周边设备和日常用品，可在下方日本商城搜索。"
rakuten_label: "在乐天市场搜索"
amazon_label: "在 Amazon.co.jp 搜索"
shop_disclosure: "本站使用广告（Google AdSense）和联盟计划（乐天联盟、Amazon 联盟）。作为 Amazon 联盟成员，本站通过符合条件的购买获得收入。"
---
