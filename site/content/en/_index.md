---
title: "Lite Simple Rust Browser"
description: "A light, simple web browser written from scratch in Rust with low memory use as its first goal."
tagline: "Light and simple. A web browser written in Rust."
lead: "It parses HTML and CSS, lays out, paints, fetches and runs JavaScript with its own engine, so it stays light however many tabs you open."
cta_download: "Get it"
cta_source: "Source code"
stats_title: "Memory use (measured on a release build)"
stats:
  - { value: "~22 MB", label: "Blank tab" }
  - { value: "~65 MB", label: "One Wikipedia article" }
  - { value: "~185 MB", label: "Ten tabs" }
features_title: "Features"
features:
  - { title: "Low memory", body: "Background tabs are frozen and discarded automatically to stay within a memory budget." }
  - { title: "Three processes", body: "Browser, Content and Network run separately. The window stays responsive on heavy pages, and Content restarts itself if it stops." }
  - { title: "JavaScript", body: "Runs the DOM, events, timers, fetch and more on the Boa engine." }
  - { title: "Load levels 1-5", body: "From \"Minimal\" to \"Maximum\". Image loading can be switched on or off separately." }
  - { title: "Password manager", body: "Asks to save when you sign in and fills it in next time. Passwords are encrypted, with the key kept in the Windows Credential Manager." }
  - { title: "Bookmarks", body: "Ctrl+D to add, Ctrl+B for the list. Choose your home page in Settings." }
  - { title: "History and downloads", body: "Ctrl+H for history, Ctrl+J for downloads. Links to files are saved without leaving the page, with live progress." }
  - { title: "Five display languages", body: "Japanese, English, Simplified Chinese, Korean and Spanish." }
  - { title: "Content blocking", body: "Block ads and other unwanted requests with filter lists." }
start_title: "Getting started"
start:
  - "A new tab opens at startup. Type a URL or a search in the box at the top (Ctrl+L)."
  - "Ctrl+T new tab / Ctrl+W close / Alt+Left back / Alt+Home home / Ctrl+click open in background"
  - "Settings: about:settings, passwords: about:passwords, memory: about:memory."
download_button: "Download from Google Drive"
download_title: "Get it"
download_body: "For Windows 11 (64-bit). Run the installer, or just unzip the package. History and settings are stored in %LOCALAPPDATA%\\Kumo. Licensed under your choice of MIT or Apache-2.0."
limits_title: "Current limitations"
limits_body: "This is a work in progress, so not every site renders perfectly. ES modules, iframes, canvas, GPU rendering and inline SVG are not supported yet."
---
