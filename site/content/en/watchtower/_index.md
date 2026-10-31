---
title: "watchtower-rs"
site_name: "watchtower-rs"
description: "A changedetection.io-compatible web page watcher in Rust. One binary, about 24 MB for 5,000 watches."
tagline: "Quietly watch pages for changes."
lead: "watchtower-rs checks web pages on a schedule and notifies you of the differences. It has a changedetection.io-compatible REST API and runs as a single Rust binary."
pills: ["Single binary", "SQLite", "changedetection.io compatible", "REST API"]
nav_all: "All apps"
nav_features: "Features"
nav_start: "Getting started"
nav_get: "Download"
nav_status: "Status"
stats_title: "Memory use (measured on a Windows release build)"
stats:
  - { value: "~17 MB", label: "1,000 watches registered and idle" }
  - { value: "~24 MB", label: "Peak while watching 5,000 pages every 300 s" }
  - { value: "~90 s", label: "For the first fetch of all 5,000 pages" }
stats_note: "The executable is about 28 MB. Measured against a simulated site of my own; your results will vary with the environment."
features_title: "Features"
features:
  - { icon: "🍃", title: "Low memory", body: "Memory grows slowly as you add watches. Snapshots are compressed and kept in files." }
  - { icon: "🔁", title: "Compatible", body: "The REST API follows the original shape and authenticates with x-api-key. You can also import the original url-watches.json." }
  - { icon: "✂️", title: "Compare only what matters", body: "Narrow the target with CSS selectors, XPath, JSONPath or jq, and remove noise with regular expressions and ignored text. Notify only when trigger text appears, if you like." }
  - { icon: "🌐", title: "JavaScript pages too", body: "Connects to a separately running Chrome and fetches the rendered page. Register click, type and wait steps, or pick an element on a screenshot with the visual selector." }
  - { icon: "🏷️", title: "Price, stock and PDF", body: "Reads prices from a product page's structured data and notifies at or below your threshold. PDF text can be compared too." }
  - { icon: "🔔", title: "Many notification targets", body: "Discord, Slack, Telegram, ntfy, Gotify, Mattermost, Pushover, email and webhooks, with other services forwarded to an Apprise API. Titles and bodies can be templated." }
  - { icon: "🛠️", title: "Fine control of fetching", body: "Set headers, method, body, proxy and interval per watch, per tag or globally. The closest setting wins." }
  - { icon: "🛡️", title: "Built with safety in mind", body: "SSRF protection against internal addresses, argon2 passwords, CSRF protection and a Host name allow-list. Header values and notification secrets stay out of logs." }
  - { icon: "📈", title: "Easy to operate", body: "OpenAPI docs, a Prometheus /metrics endpoint, an RSS feed, backup and restore, and a health check. Docker files are included." }
  - { icon: "📊", title: "Dashboard", body: "See the number of watches, the failing ones, the notification queue and the latest changes on one page." }
  - { icon: "🚀", title: "One-command start", body: "watchtower ui starts the server and opens the admin page in your browser." }
how_title: "How it works"
how:
  - { title: "Fetch", body: "Pages are fetched at the interval you set. Connections to internal addresses are refused by default." }
  - { title: "Tidy", body: "The target is narrowed, unwanted elements and noisy text are removed, and the result becomes plain text." }
  - { title: "Compare and notify", body: "Differences from the last check are saved, and a notification is sent only when your conditions match. Failed notifications are retried." }
start_title: "Getting started"
start_lead: "Set a password and start it from the command line. The admin UI opens on 127.0.0.1:5000 by default."
start_code: |-
  watchtower set-password     # admin password (one line from standard input)
  watchtower api-key          # issue a REST API key
  watchtower watch add https://example.com/ --interval 300
  watchtower ui               # start and open the admin page in a browser (run does not open it)
start_note: "Configure it with environment variables (WATCHTOWER_DATA_DIR, WATCHTOWER_LISTEN, WATCHTOWER_BASE_URL and more) or a config file."
download_title: "Download"
download_button: "Download from Google Drive"
download_body: "A zip for Windows 11 (64-bit), about 11 MB. Extract it and run watchtower.exe; there is no installer. Static Linux binaries (x86_64 and aarch64) are on the GitHub releases page."
cta_source: "Source code"
sha_label: "SHA-256 (zip)"
release_label: "Linux binaries and source (GitHub)"
download_url: "https://drive.google.com/drive/folders/1349ZQeTtuH27DqxitP2U3XoAaON8HTG0?usp=sharing"
repo_url: "https://github.com/taotaro1104-commits/watchtower-rs"
release_url: "https://github.com/taotaro1104-commits/watchtower-rs/releases"
download_file: "watchtower-rs-0.1.0-windows-x64.zip"
download_sha: "398abd5f1c571416bcd61d00f63711b1313fffbf5022b766fa0ba8810a10e3a9"
status_title: "Status"
status_body: "In development (v0.1.0, pre-release). It has been run as a real binary on Windows 11 and driven through the browser. The source code is on GitHub."
todo_title: "Not verified yet"
todo:
  - "The Docker image (it builds in CI, but has not been run yet)"
  - "Static Linux (musl) and aarch64 binaries (they build, but have not been run yet)"
  - "Sending through a real mail server with TLS or STARTTLS"
footer: "© watchtower-rs — Apache-2.0"
---
