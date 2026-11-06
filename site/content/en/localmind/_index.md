---
title: "LocalMind"
site_name: "LocalMind"
description: "An Android app written in Rust that transcribes recordings, writes meeting minutes, searches your documents and answers questions with AI, all on the device. Your recordings and documents are never sent out."
tagline: "Meetings and documents, kept on your device."
lead: "LocalMind transcribes recordings, writes meeting minutes, searches your own documents and answers your questions with AI, all inside your smartphone. Recordings, transcripts and documents never leave the device."
pills: ["Android", "Works offline", "Rust", "Japanese & English screens"]
nav_all: "All apps"
nav_features: "Features"
nav_how: "How to use"
nav_get: "Get it"
nav_status: "Status"
stats_title: "What it does"
stats:
  - { value: "3 features", label: "Meeting minutes, document search and AI questions" }
  - { value: "On the device", label: "Recordings, transcripts and documents are never sent out" }
  - { value: "3 model sizes", label: "Pick an AI model by size, and switch later" }
stats_note: "The AI models (about 0.5 to 2.1 GB) are not bundled with the app. You download one the first time you need it, after you agree. A smaller model is faster, but its minutes and answers are less accurate."
shots_title: "Screenshots"
shots_note: "Screens were captured in an Android 15 emulator (Japanese screens), using sample documents and a short audio clip of a pretend meeting."
shots:
  - { img: "search.jpg", caption: "Document search, with sources" }
  - { img: "groups.jpg", caption: "Document groups" }
  - { img: "minutes.jpg", caption: "Minutes, with the time of the cited speech" }
  - { img: "ask.jpg", caption: "Asking the AI" }
  - { img: "model.jpg", caption: "Downloading an AI model" }
features_title: "Features"
features:
  - { icon: "🎙️", title: "Record and transcribe", body: "Record meetings and lectures on the device only. You can confirm everyone's consent before recording, and a notification is always shown while recording. Transcription runs on the device (whisper.cpp)." }
  - { icon: "📝", title: "Minutes with sources", body: "From the transcript it writes a summary, decisions, action items and open issues. Each item carries the time of the speech it is based on, and items whose source cannot be confirmed are left out." }
  - { icon: "🔎", title: "Document search in Japanese", body: "Register text or Markdown documents and search them, even with a reworded question. It shows where the answer is, with the document, heading and line numbers, and says \"not found\" when there is no answer." }
  - { icon: "🌐", title: "Import a document from a URL", body: "Enter a page URL and only its main text is taken as a document. It fetches public https pages only, and never reads from your own device or an internal network." }
  - { icon: "🗂️", title: "Document groups", body: "Put documents into groups and switch the list between them. You can limit a search to the chosen group, move documents between groups, or delete a whole group at once." }
  - { icon: "💬", title: "Ask the AI", body: "Ask the AI a question without loading any document. It answers on the device alone, or, only when you choose, searches Wikipedia first. The search words are shown to you before they are sent." }
  - { icon: "🔁", title: "Choose and switch AI models", body: "Pick from Qwen2.5 0.5B, 1.5B and 3B, or give a URL for another model (GGUF). Downloaded models stay on the device, and you can switch between them or delete them at any time." }
  - { icon: "🌍", title: "Japanese and English screens", body: "The app's screens are available in Japanese and English and follow the device language (on Android 13 and later you can also choose it per app in Settings). The AI answers a question in the language it was asked in." }
privacy_title: "How your data is handled"
privacy_body: "Recordings, transcripts and documents are never sent out. The app uses the network in only three cases, and each one is something you chose to do."
privacy:
  - "Downloading an AI model (the first time you use it, after you check where it comes from)"
  - "Choosing \"search Wikipedia\" when asking the AI (only the short search words the AI made are sent, after you confirm them. Your whole question and your documents are not sent)"
  - "Using \"Add from URL\" for a document (it connects once, to the URL you entered)"
how_title: "How to use"
how:
  - { title: "Get an AI model", body: "The first time you make minutes or ask a question, choose a model (or enter a URL) and download it. Wi-Fi is recommended." }
  - { title: "Record and make minutes", body: "Record on the recording screen, stop, and run the transcription. Then tap \"Make minutes\" on the result screen and the minutes are ready in a few minutes. You can share them." }
  - { title: "Add documents, then search or ask", body: "On the search screen, add documents from text, a file or a URL, and type a question to see the passages that answer it. To ask the AI without any documents, use the question screen." }
download_title: "Get it"
download_body: "The Android APK (the app itself) is shared on Google Drive. The source code is on GitHub, and you can also build it yourself."
download_note: "The APK is signed with a key we generated ourselves, so your device will ask you to allow apps from unknown sources. It is about 700 MB, so downloading over Wi-Fi is recommended. The AI models are not included; you download one separately the first time you use it."
download_button: "Download from Google Drive"
download_url: "https://drive.google.com/drive/folders/1hfZGFiTYGIdArpSbNyXrt-hWH11323Wv?usp=drive_link"
android_file: "LocalMind-0.1.apk"
android_sha: "078f7c82afc78e46455492537aa14018be75717d00c0895ee2696f5b1c668224"
sha_label: "SHA-256 (v0.1)"
cta_source: "Source code (GitHub)"
repo_url: "https://github.com/taotaro1104-commits/localmind"
status_title: "Status"
status_body: "Under development (v0.1, prototype). In an Android 15 emulator we have confirmed the whole flow from recording to minutes, document search and import, and asking the AI. AI answers can be wrong, so please check anything important yourself."
todo_title: "Not yet verified, or not supported"
todo:
  - "Real devices and their speed (only checked in an emulator; the speed on an arm64 device is not known yet)"
  - "The accuracy of small models (the sorting of minutes and the answers can still be rough)"
  - "Long meetings (the transcript used for minutes is limited to 5,000 characters)"
  - "Importing PDFs, and reading text from images (OCR), are not supported"
  - "Distribution on Google Play (the app is about 700 MB, over the size limit)"
  - "Transcription and minutes of English meetings (transcription and minutes are tuned for Japanese for now)"
footer: "© LocalMind"
---
