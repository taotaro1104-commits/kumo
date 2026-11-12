{
  "description": "Rust EQ is an Android music player with 16 presets, a 10/31-band equalizer, virtual bass and spatial effects. Shape the sound of music stored on your device.",
  "tagline": "Your favorite tracks.\nYour kind of sound.",
  "lead": "Start with a preset. Add a little bass or bring vocals forward. Rust EQ lets you find a balance you like while listening to music stored on your Android device.",
  "pills": [
    "Android 10+",
    "Offline playback",
    "In development v0.4.8",
    "日本語 / English"
  ],
  "nav_all": "All apps",
  "nav_features": "Features",
  "nav_shots": "Screens",
  "nav_get": "Download",
  "download_button": "Get it on Google Drive",
  "source_button": "View source code",
  "hero_caption": "See the balance as an EQ curve.",
  "features_title": "A quick start. Room to fine-tune.",
  "features": [
    {
      "title": "16 presets to suit your mood",
      "body": "Start with Flat, Vocal, Rich Bass and more. 31-band presets such as Rock and Classical are now included. Small curve previews help you compare their sound profiles."
    },
    {
      "title": "Shape the lows, mids and highs",
      "body": "Adjust frequency, gain and Q with a 10/31-band parametric EQ. Boost virtual bass up to 200%, and use a compressor, a limiter, an echo (level, delay, repeats), reverb, a dynamic bass EQ and a true-peak limiter, too."
    },
    {
      "title": "Explore width and space",
      "body": "Adjust stereo width and crossfeed. Experiment with headphone spatial sound using HRTF/SOFA and IR reverb."
    },
    {
      "title": "Keep the sound you find",
      "body": "Save custom presets and import or export them as JSON. Import AutoEQ text and CSV curves for fitting, too."
    }
  ],
  "shots_title": "Three screens. Clear controls.",
  "shots_note": "Controls are grouped into Sound, Space and Tools. These Android emulator screenshots show the Japanese app interface. Select an image to open it at full size.",
  "shots": [
    {
      "img": "sound.png",
      "caption": "Sound",
      "body": "Presets and the EQ curve, together."
    },
    {
      "img": "space.png",
      "caption": "Space",
      "body": "A speaker diagram helps explain stereo width."
    },
    {
      "img": "tools.png",
      "caption": "Tools",
      "body": "Save and import your settings in one place."
    },
    {
      "img": "presets.png",
      "caption": "Preset library",
      "body": "Compare curves and pick a starting point."
    }
  ],
  "how_title": "Start with a track you love.",
  "how": [
    {
      "title": "Open your music",
      "body": "Choose a track using the Android file picker. You can also open a folder and add its tracks to the queue."
    },
    {
      "title": "Find your balance",
      "body": "Pick a preset and play. Toggle EQ on and off to compare, then make small adjustments as you listen."
    },
    {
      "title": "Save your settings",
      "body": "Keep a favorite setup in Tools. Playback, pause and track controls are also available in the notification."
    }
  ],
  "download_title": "Try Rust EQ",
  "download_body": "Get the signed Android APK from the Google Drive distribution folder. The 0.4.8 filename is rust-eq-0.4.8-release.apk.",
  "download_note": "Android 10+ / arm64-v8a and x86_64. Save the APK and follow your device’s installation prompts. The interface supports English and Japanese and follows your language preferences. Android 13+ also offers per-app language settings. Check the filename and version available in the folder before downloading.",
  "details_title": "Before you start",
  "details": [
    {
      "title": "Supported music files",
      "body": "Plays mono and stereo MP3, WAV, FLAC, Ogg Vorbis, AAC-LC/M4A, ALAC and AIFF audio. DRM-protected music is not supported."
    },
    {
      "title": "Can it apply EQ to other apps?",
      "body": "Select and save a player, then tap Connect. Players that do not announce audio sessions may work with optional Enhanced detection and additional ADB setup. Check the target’s actual connection status. Device EQ supports compatible audio sessions; it does not force all app audio through Rust DSP. Virtual bass, dynamics, HRTF and IR are for local playback."
    },
    {
      "title": "Why is track information missing?",
      "body": "Select YouTube Music and allow Android notification access. This is separate from ordinary notification permission and the optional DUMP setup for EQ discovery. On Android 13+, check App info → menu → Allow restricted settings if access is blocked. A player must publish media information for it to appear."
    },
    {
      "title": "Your music and settings",
      "body": "The app does not request internet permission. It asks for the microphone only when you start room calibration, and that audio is never saved or sent. Selected music is processed on your device, and files are opened through the Android file picker."
    },
    {
      "title": "Development and testing",
      "body": "v0.4.8 adds listening and tone-matching tools plus reverb and more. All 52 Rust tests, all 54 Android tests on an Android 15 emulator, Lint and APK signing were verified, but physical-device testing and listening checks have not been done. v0.4.5 passed 42 Android 13 integration tests, 15 unit tests and 35 Rust tests. We verified same-key upgrades, saved settings and live metadata recovery after process termination. The current YouTube Music client on physical hardware has not been verified."
    }
  ],
  "footer": "Sound made yours. Rust EQ",
  "listing_tagline": "Find your kind of sound.",
  "listing_body": "Shape local music with 16 presets and a 10/31-band EQ. Explore virtual bass and spatial effects with clear, friendly controls.",
  "listing_tags": [
    "Android 10+",
    "Offline playback",
    "Rust DSP"
  ],
  "listing_cta": "Meet Rust EQ",
  "title": "Rust EQ",
  "site_name": "Rust EQ",
  "type": "rusteq",
  "download_url": "https://drive.google.com/drive/folders/1hImCSXhyR_w83_6F2caRq50dvMjMIgfF?usp=drive_link",
  "repo_url": "https://github.com/taotaro1104-commits/RustAndroidequalizer",
  "og_image": "rusteq/og-v2.png",
  "og_image_width": 1731,
  "og_image_height": 909,
  "og_image_alt": "Rust EQ, an Android music player and equalizer with 16 presets, 10/31-band EQ and offline playback.",
  "external_title": "Keep your EQ close, in other apps too.",
  "external_lead": "Choose compatible apps and save them. When a target is disconnected, tap Connect to open your player.",
  "external_steps": [
    {
      "title": "Choose your apps",
      "body": "Search by app name and select the players you want to use with EQ. You can choose several."
    },
    {
      "title": "Save your choices",
      "body": "Keep your selected apps and listener settings for next time."
    },
    {
      "title": "Tap to connect",
      "body": "Tap Connect, then start or reopen a track in the player. Use Reconnect to retry an active session."
    }
  ],
  "external_note": "Applies device EQ to compatible apps that share their audio session. Resume after restart can restore the listener. It cannot force every app’s audio through Rust DSP.",
  "external_shots": [
    {
      "img": "target-picker-en.png",
      "caption": "Choose apps and save"
    },
    {
      "img": "target-connected-en.png",
      "caption": "See the connection and reconnect"
    }
  ],
  "playback_title": "See what is playing in YouTube Music.",
  "playback_lead": "Show the selected player’s title, artist and playback state in Sound and Playback details.",
  "playback_steps": [
    {
      "title": "Select YouTube Music",
      "body": "Choose and save YouTube Music in Tools → EQ for other apps. YouTube Music and YouTube are separate apps."
    },
    {
      "title": "Allow notification access",
      "body": "Open Playback details → Set up playback information access, then allow notification access in Android settings."
    },
    {
      "title": "Play a track and return",
      "body": "Start music in YouTube Music, then return to Rust EQ. Tap Refresh playback information if needed."
    }
  ],
  "playback_note": "Track information and EQ connection have separate status indicators. Notification bodies are not analyzed and no listening history is saved. Phone EQ does not apply to audio playing on a cast device."
}
