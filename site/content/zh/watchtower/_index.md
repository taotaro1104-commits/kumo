---
title: "watchtower-rs"
site_name: "watchtower-rs"
description: "用 Rust 实现、与 changedetection.io 兼容的网页监视工具。单一可执行文件，监视 5,000 个页面约 24 MB。"
tagline: "安静地盯着页面的变化。"
lead: "watchtower-rs 会定期检查网页，并在内容变化时通知差异。它提供与 changedetection.io 兼容的 REST API，以单个 Rust 可执行文件运行。"
pills: ["单一可执行文件", "SQLite", "兼容 changedetection.io", "REST API"]
nav_all: "全部应用"
nav_features: "特点"
nav_start: "使用方法"
nav_get: "下载"
nav_status: "开发状态"
stats_title: "内存占用（Windows 发布版实测）"
stats:
  - { value: "约 17 MB", label: "登记 1,000 个并处于空闲时" }
  - { value: "约 24 MB", label: "每 300 秒监视 5,000 个页面时的峰值" }
  - { value: "约 90 秒", label: "5,000 个页面完成首次抓取" }
stats_note: "可执行文件约 28 MB。测试对象是自建的模拟站点，实际结果因环境而异。"
features_title: "特点"
features:
  - { icon: "🍃", title: "省内存", body: "登记再多监视项，内存也不易增加。快照压缩后保存为文件。" }
  - { icon: "🔁", title: "与原版兼容", body: "REST API 沿用原版的形式，用 x-api-key 认证，也可导入原版的 url-watches.json。" }
  - { icon: "✂️", title: "只比较需要的部分", body: "用 CSS 选择器、XPath、JSONPath、jq 缩小范围，用正则和忽略文本去除干扰。也可以只在触发文本出现时通知。" }
  - { icon: "🌐", title: "支持 JavaScript 页面", body: "连接另行运行的 Chrome，获取渲染后的页面。可登记点击、输入、等待等步骤，还能在截图上选择元素（可视化选择器）。" }
  - { icon: "🏷️", title: "价格、库存与 PDF", body: "从商品页的结构化数据读取价格，低于阈值时通知，也能比较 PDF 的正文。" }
  - { icon: "🔔", title: "丰富的通知渠道", body: "支持 Discord、Slack、Telegram、ntfy、Gotify、Mattermost、Pushover、邮件和 Webhook，其他服务转发到 Apprise API。标题和正文可用模板自定义。" }
  - { icon: "🛠️", title: "精细的抓取设置", body: "可按监视项、标签或全局设置请求头、方法、请求体、代理和间隔，就近的设置优先。" }
  - { icon: "🛡️", title: "重视安全", body: "防止访问内部地址的 SSRF 防护、argon2 密码、CSRF 防护、Host 名称白名单。请求头的值和通知密钥不会写入日志。" }
  - { icon: "📈", title: "便于运维", body: "提供 OpenAPI 文档、Prometheus 格式的 /metrics、RSS 订阅、备份与恢复、健康检查，并附带 Docker 文件。" }
  - { icon: "📊", title: "仪表盘", body: "在一个页面里查看监控数量、失败项、通知队列和最近的变化。" }
  - { icon: "🚀", title: "一条命令启动", body: "watchtower ui 会启动服务器，并在浏览器中打开管理页面。" }
how_title: "工作方式"
how:
  - { title: "抓取", body: "按设定的间隔抓取页面。默认拒绝连接内部地址。" }
  - { title: "整理", body: "缩小对象范围，去除不需要的元素和易变的文字，转为文本。" }
  - { title: "比较并通知", body: "与上次有差异就保存，仅在满足条件时发送通知。发送失败的通知会重试。" }
start_title: "使用方法"
start_lead: "在命令行设置密码后启动。管理界面默认在 127.0.0.1:5000 打开。"
start_code: |-
  watchtower set-password     # 管理页面的密码（从标准输入读取一行）
  watchtower api-key          # 生成 REST API 密钥
  watchtower watch add https://example.com/ --interval 300
  watchtower ui               # 启动并在浏览器中打开管理页面（run 不会打开页面）
start_note: "通过环境变量（WATCHTOWER_DATA_DIR、WATCHTOWER_LISTEN、WATCHTOWER_BASE_URL 等）或配置文件进行设置。"
download_title: "下载"
download_button: "从 Google 云端硬盘下载"
download_body: "适用于 Windows 11（64 位）的 zip，约 11 MB。解压后运行 watchtower.exe 即可，无需安装。Linux（x86_64 与 aarch64）的静态二进制文件在 GitHub 的发布页面。"
cta_source: "源代码"
sha_label: "SHA-256（zip）"
release_label: "Linux 二进制文件与源代码（GitHub）"
download_url: "https://drive.google.com/drive/folders/1349ZQeTtuH27DqxitP2U3XoAaON8HTG0?usp=sharing"
repo_url: "https://github.com/taotaro1104-commits/watchtower-rs"
release_url: "https://github.com/taotaro1104-commits/watchtower-rs/releases"
download_file: "watchtower-rs-0.1.0-windows-x64.zip"
download_sha: "398abd5f1c571416bcd61d00f63711b1313fffbf5022b766fa0ba8810a10e3a9"
status_title: "开发状态"
status_body: "开发中（v0.1.0，预发布版）。已在 Windows 11 上运行实际的二进制文件，并通过浏览器操作验证。源代码已在 GitHub 公开。"
todo_title: "尚未验证的内容"
todo:
  - "Docker 镜像（CI 中可以构建，但尚未运行验证）"
  - "Linux 静态二进制文件（musl）与 aarch64（可以构建，但尚未运行验证）"
  - "通过使用 TLS 或 STARTTLS 的真实邮件服务器发送"
footer: "© watchtower-rs — Apache-2.0"
---
