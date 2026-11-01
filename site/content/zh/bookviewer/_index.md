---
title: "RustHighBookViewer"
site_name: "RustHighBookViewer"
description: "用 Rust 编写的漫画与图片阅读器。可阅读 ZIP/CBZ、7z、RAR、EPUB 和图片文件夹，支持从右向左的双页等。支持 Android 和 Windows。"
tagline: "打开就能读。"
lead: "RustHighBookViewer 是用来打开并阅读漫画、图片压缩包和文件夹的阅读器。支持从右向左的双页、缩放、调色，以及记住读到的位置，可在 Android 和 Windows 上使用。"
pills: ["Android", "Windows", "ZIP / CBZ / 7z / RAR / EPUB", "支持右开本"]
nav_all: "全部应用"
nav_features: "特点"
nav_how: "使用方法"
nav_get: "下载"
nav_status: "开发状态"
stats_title: "支持的内容"
stats:
  - { value: "5 种", label: "可打开 ZIP/CBZ、7z、RAR、EPUB 和图片文件夹" }
  - { value: "4 种显示", label: "单页、双页、自动双页、纵向滚动" }
  - { value: "2 个平台", label: "Android 8.0 及以上，以及 Windows" }
stats_note: "支持 RAR 4.x / 5.x（未加密）。7z 和 RAR 在打开时会先解压到设备的存储空间。EPUB 支持每页为一张图片的书（漫画、写真集）。"
features_title: "特点"
features:
  - { icon: "📖", title: "右开本、左开本都行", body: "可切换装订方向，双页的排列也随之改变。封面可单独显示，横向的页面单页显示。" }
  - { icon: "🔍", title: "缩放与放大镜", body: "双指捏合或双击即可缩放并平移。Android 版还有放大镜，可把手指划过的地方放大 2 倍。" }
  - { icon: "🗂️", title: "页面一览", body: "从全书的缩略图中直接跳到想读的页面（Android 版）。" }
  - { icon: "🎨", title: "调色", body: "亮度、对比度、伽马，以及黑白、复古棕、反色。设置会按每本书分别记住（Android 版）。" }
  - { icon: "⏱️", title: "自动翻页", body: "每 3、5、10、20 秒自动翻页，到最后一页停止（Android 版）。" }
  - { icon: "📚", title: "书架与继续阅读", body: "带有历史、收藏和标题搜索的书架。关闭时会保存读到的位置，下次可从上次的位置继续。" }
  - { icon: "📁", title: "也能打开文件夹", body: "选择文件夹后，每个文件夹里的图片合为一本，EPUB、ZIP、7z、RAR 则每个文件各算一本，子文件夹也一并处理，全部加入书架。在 Android 上允许“所有文件访问权限”后，可直接选择 Download 等文件夹。" }
  - { icon: "🛡️", title: "注重安全", body: "对解压的大小和文件数量设有上限，可用空间不足时会中途停止。压缩包内含 ../ 之类的路径会被忽略。" }
  - { icon: "🔢", title: "页面跳转与书签", body: "输入页码即可跳转，也可从书签列表跳转。一键添加或取消书签（Android 版）。" }
  - { icon: "⚙️", title: "设置", body: "可设定默认的显示方式、装订方向、适应方式、音量键翻页、保持亮屏和书架排序，以及图像缓存可使用的内存（Android 版）。" }
  - { icon: "📘", title: "EPUB 漫画", body: "可打开每页为一张图片的 EPUB，并按阅读顺序排列。不支持以文字为主的 EPUB。" }
  - { icon: "⚡", title: "再次打开更快", body: "会记住页面尺寸，因此文件夹和压缩包从第二次起打开更快。" }
how_title: "使用方法"
how:
  - { title: "打开", body: "在 Android 上，从书架选择“文件”或“文件夹”。在 Windows 上，把文件或文件夹拖放到 exe 上。" }
  - { title: "阅读", body: "点按屏幕左侧或右侧翻页。点按中央会出现菜单，可更改显示方式和装订方向。" }
  - { title: "继续阅读", body: "关闭时会保存读到的位置。从书架选择这本书即可继续。" }
download_title: "下载"
download_button: "从 Google 云端硬盘下载"
download_body: "Android 用的 APK 和 Windows 用的 exe 都通过 Google 云端硬盘提供。没有安装程序，exe 直接运行即可使用。"
download_note: "APK 使用我们自己生成的密钥签名，因此设备会要求允许安装来自未知来源的应用。exe 没有签名，Windows 的 SmartScreen 可能会给出警告。"
cta_source: "源代码"
sha_label: "SHA-256（v0.1.3）"
release_label: "更新记录与源代码（GitHub）"
download_url: "https://drive.google.com/drive/folders/15qKnVY5FglIUeVdFlwyLt3eVNU_XtltK?usp=sharing"
repo_url: "https://github.com/taotaro1104-commits/RustHighBookViewer"
release_url: "https://github.com/taotaro1104-commits/RustHighBookViewer/releases"
android_file: "RustHighBookViewer-0.1.3-android.apk"
windows_file: "RustHighBookViewer-desktop.exe"
android_sha: "2bef5262f6cbb348b7961bee0426102f33318eae24f88c17bb6ad4bf15d6d5fc"
windows_sha: "5211618dec0cff0fc5f1179d5a7de8f4f4556fc6baadc59ac90c4b095672854a"
status_title: "开发状态"
status_body: "开发中（v0.1.3，预发布版）。在 Android 上，已在 Android 15 模拟器中确认 7z、RAR、EPUB 和文件夹批量导入可用。源代码已在 GitHub 公开。"
todo_title: "尚未验证或不支持的内容"
todo:
  - "Android 真机上的运行情况（目前只在模拟器中确认）"
  - "Windows 版的界面外观和操作（只确认了能启动并持续运行）"
  - "带密码的压缩包，以及分成多个文件的 RAR（不支持）"
footer: "© RustHighBookViewer — MIT License"
---
