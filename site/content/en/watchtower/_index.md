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
how_title: "How it works"
how:
  - { title: "Fetch", body: "Pages are fetched at the interval you set. Connections to internal addresses are refused by default." }
  - { title: "Tidy", body: "The target is narrowed, unwanted elements and noisy text are removed, and the result becomes plain text." }
  - { title: "Compare and notify", body: "Differences from the last check are saved, and a notification is sent only when your conditions match. Failed notifications are retried." }
start_title: "Getting started"
start_lead: "Set a password and start it from the command line. The admin UI opens on 127.0.0.1:5000 by default."
start_code: |-
  watchtower set-password     # UI password (one line from standard input)
  watchtower api-key          # issue a REST API key
  watchtower watch add https://example.com/ --interval 300
  watchtower run              # start scheduled checks and the admin UI
start_note: "Configure it with environment variables (WATCHTOWER_DATA_DIR, WATCHTOWER_LISTEN, WATCHTOWER_BASE_URL and more) or a config file."
status_title: "Status"
status_body: "Under development. Downloads and the source code are not published yet. It has been tested on Windows 11."
todo_title: "Not verified yet"
todo:
  - "Building and running the Docker image"
  - "A static Linux binary (musl) and aarch64"
  - "Sending through a real mail server over TLS or STARTTLS"
footer: "© watchtower-rs — Apache-2.0"
---
