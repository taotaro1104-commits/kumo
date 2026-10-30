---
title: "Lite Simple Rust Browser"
description: "A light, simple web browser written from scratch in Rust with low memory use as its first goal."
tagline: "Light and simple. A web browser written in Rust."
lead: "It parses HTML and CSS, lays out, paints, fetches and runs JavaScript with its own engine, so it stays light however many tabs you open."
nav_features: "Features"
nav_get: "Get it"
cta_source: "Source code"
ad_label: "Ad"
stats_title: "Memory use (measured on a release build)"
stats:
  - { mb: 22, value: "~22 MB", label: "Blank tab" }
  - { mb: 65, value: "~65 MB", label: "One Wikipedia article" }
  - { mb: 185, value: "~185 MB", label: "Ten tabs" }
features_title: "Features"
features:
  - { icon: "🍃", title: "Low memory", body: "Background tabs are frozen and discarded automatically to stay within a memory budget." }
  - { icon: "🔀", title: "Three processes", body: "Browser, Content and Network run separately. The window stays responsive on heavy pages, and Content restarts itself if it stops." }
  - { icon: "⚡", title: "JavaScript", body: "Runs the DOM, events, timers, fetch and more on the Boa engine." }
  - { icon: "🎚️", title: "Load levels 1-5", body: "From \"Minimal\" to \"Maximum\". Image loading can be switched on or off separately." }
  - { icon: "🔑", title: "Password manager", body: "Asks to save when you sign in and fills it in next time. Passwords are encrypted, with the key kept in the Windows Credential Manager." }
  - { icon: "⭐", title: "Bookmarks", body: "Ctrl+D to add, Ctrl+B for the list. Choose your home page in Settings." }
  - { icon: "🕘", title: "History and downloads", body: "Ctrl+H for history, Ctrl+J for downloads. Links to files are saved without leaving the page, with live progress." }
  - { icon: "🔍", title: "Page zoom", body: "Ctrl+plus, Ctrl+minus or Ctrl+wheel from 25% to 500%. Ctrl+0 resets it." }
  - { icon: "あ", title: "Japanese input", body: "Text converted with an IME goes into the address bar and page fields." }
  - { icon: "🌏", title: "Five display languages", body: "Japanese, English, Simplified Chinese, Korean and Spanish." }
  - { icon: "🛡️", title: "Content blocking", body: "Block ads and other unwanted requests with filter lists." }
start_title: "Getting started"
start_lead: "A new tab opens at startup. Type a URL or a search in the box at the top."
keys:
  - { k: "Ctrl L", d: "Go to the address bar" }
  - { k: "Ctrl T", d: "New tab" }
  - { k: "Ctrl W", d: "Close tab" }
  - { k: "Ctrl D", d: "Bookmark this page" }
  - { k: "Ctrl H", d: "History" }
  - { k: "Ctrl J", d: "Downloads" }
  - { k: "Ctrl +/-", d: "Zoom in or out" }
start_note: "See about:settings for settings, about:passwords for saved passwords and about:memory for memory use."
download_title: "Get it"
download_button: "Download from Google Drive"
recommended: "Recommended"
sha_summary: "Show checksums (SHA-256)"
download_body: "For Windows 11 (64-bit). Run the installer, or just unzip the package. History and settings are stored in %LOCALAPPDATA%\\Kumo. Licensed under your choice of MIT or Apache-2.0. For Android 8.0 or later (allow installs from unknown sources)."
release_title: "Latest release"
release_version: "Version 0.1.0 (2026-10-29)"
release_files:
  - { name: "kumo-0.1.0-windows-x64-setup.exe", kind: "Installer", size: "8.8 MB", sha: "2f2a017fdaf6d8f85c7e70e497facdd4a00bfc2f6081c84974105158da1d1e84" }
  - { name: "kumo-0.1.0-windows-x64.zip", kind: "Zip (just unzip and run)", size: "10.4 MB", sha: "d78c7542a9a61225636428c30e1b2fc8d30df193cc19554963bbc030317bdba8" }
  - { name: "kumo-0.1.0-android-arm64.apk", kind: "Android app (arm64)", size: "12.1 MB", sha: "68bb339dc891b375dd2fcc8f3ae131e8fa8a32a42c1517e2316f92eda4b29478" }
sha_note: "Compare the SHA-256 of the file you downloaded with the values below."
changes_title: "Recent changes"
changes:
  - "Android version added (touch scrolling, tap, pinch zoom, on-screen keyboard, IME composition, password saving)"
  - "Page zoom (Ctrl+plus, Ctrl+minus, Ctrl+wheel, Ctrl+0)"
  - "Japanese input (IME) support, with the composing text shown in the address bar"
  - "History (Ctrl+H) and downloads (Ctrl+J). Links to files are saved without leaving the page"
  - "Renamed to Lite Simple Rust Browser, with an original icon"
  - "Password manager and five display languages"
limits_title: "Current limitations"
limits_body: "This is a work in progress, so not every site renders perfectly. ES modules, iframes, canvas, GPU rendering and inline SVG are not supported yet. The Android version is experimental: IME composition relies on your keyboard app, and composing text is not shown inside page input fields."
shop_title: "Recommended"
shop_lead: "Shop for computers, accessories and everyday items at the Japanese stores below."
rakuten_label: "Search on Rakuten Ichiba"
amazon_label: "Search on Amazon.co.jp"
shop_disclosure: "This site uses advertising (Google AdSense) and affiliate programs (Rakuten Affiliate and Amazon Associates). As an Amazon Associate, this site earns from qualifying purchases."
---
