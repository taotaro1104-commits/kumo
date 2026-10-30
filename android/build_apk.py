"""Kumo の Android 版 APK を作る（Gradle なし）。

    python android/build_apk.py [--abi arm64-v8a --abi x86_64] [--skip-build]

必要なもの: rustup のターゲット（aarch64-linux-android など）、cargo-ndk、
Android NDK と build-tools（aapt2・zipalign・apksigner・d8）、platform の android.jar、
Java（署名用）と JDK 17（javac・d8 用。~/AndroidSdk/jdk の下か環境変数 JAVA17_HOME）。
場所は環境変数 ANDROID_HOME（既定 ~/AndroidSdk）と ANDROID_NDK_HOME（既定 $ANDROID_HOME/android-ndk-r27c）。
署名は ~/.android/debug.keystore（なければ keytool で作る）。配布用の鍵ではない。
"""

import argparse
import os
import shutil
import subprocess
import sys
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
HERE = ROOT / "android"
OUT = ROOT / "dist"
LIB_NAME = "libkumo.so"
VERSION = "0.1.0"
MIN_SDK = "26"
TARGET_SDK = "34"
BUILD_TOOLS = "35.0.0"
PLATFORM = "android-35"
DEBUG_KEYSTORE = Path.home() / ".android" / "debug.keystore"
ABI_NAMES = {"arm64-v8a": "arm64", "x86_64": "x86_64"}


def sdk_paths():
    home = Path(os.environ.get("ANDROID_HOME", Path.home() / "AndroidSdk"))
    ndk = Path(os.environ.get("ANDROID_NDK_HOME", home / "android-ndk-r27c"))
    tools = home / "build-tools" / BUILD_TOOLS
    return home, ndk, tools, home / "platforms" / PLATFORM / "android.jar"


def run(cmd, **kwargs):
    print("+", " ".join(str(c) for c in cmd), flush=True)
    subprocess.run([str(c) for c in cmd], check=True, **kwargs)


def build_libs(abis, ndk, home):
    env = dict(os.environ, ANDROID_NDK_HOME=str(ndk), ANDROID_HOME=str(home))
    cmd = ["cargo", "ndk", "--platform", MIN_SDK, "-o", ROOT / "target" / "android-libs"]
    for abi in abis:
        cmd += ["-t", abi]
    cmd += ["build", "-p", "kumo-android", "--release"]
    run(cmd, cwd=ROOT, env=env)


def ensure_keystore():
    if DEBUG_KEYSTORE.exists():
        return
    DEBUG_KEYSTORE.parent.mkdir(parents=True, exist_ok=True)
    run([
        "keytool", "-genkeypair", "-keystore", DEBUG_KEYSTORE, "-storepass", "android",
        "-keypass", "android", "-alias", "androiddebugkey", "-keyalg", "RSA", "-keysize", "2048",
        "-validity", "10000", "-dname", "CN=Android Debug,O=Android,C=US",
    ])


def jdk17(home):
    """JDK 17 の場所（javac と d8 用）。"""
    if "JAVA17_HOME" in os.environ:
        return Path(os.environ["JAVA17_HOME"])
    found = sorted((home / "jdk").glob("jdk-17*"))
    if not found:
        sys.exit("JDK 17 が見つかりません（JAVA17_HOME を設定してください）")
    return found[-1]


def build_dex(home, tools, android_jar, work):
    """android/java の Java（変換入力の ImeBridge と、鍵の KeyVault）を classes.dex にする。"""
    jdk = jdk17(home)
    classes = work / "classes"
    classes.mkdir()
    sources = sorted((HERE / "java").rglob("*.java"))
    run([
        jdk / "bin" / "javac.exe", "--release", "8", "-Xlint:-options",
        "-cp", android_jar, "-d", classes, *sources,
    ])
    class_files = sorted(classes.rglob("*.class"))
    out = work / "dex"
    out.mkdir()
    run([
        jdk / "bin" / "java.exe", "-cp", tools / "lib" / "d8.jar", "com.android.tools.r8.D8",
        "--release", "--min-api", MIN_SDK, "--lib", android_jar, "--output", out, *class_files,
    ])
    return out / "classes.dex"


def package(abis, home, tools, android_jar):
    work = ROOT / "target" / "android-apk"
    shutil.rmtree(work, ignore_errors=True)
    work.mkdir(parents=True)
    compiled = work / "compiled"
    compiled.mkdir()
    aapt2 = tools / "aapt2.exe"
    run([aapt2, "compile", "--dir", HERE / "res", "-o", compiled])
    base = work / "base.apk"
    run([
        aapt2, "link", "-o", base, "-I", android_jar,
        "--manifest", HERE / "AndroidManifest.xml",
        "--min-sdk-version", MIN_SDK, "--target-sdk-version", TARGET_SDK,
        "--version-code", "1", "--version-name", VERSION,
        *sorted(compiled.glob("*.flat")),
    ])
    dex = build_dex(home, tools, android_jar, work)
    unaligned = work / "unaligned.apk"
    with zipfile.ZipFile(base) as src, zipfile.ZipFile(unaligned, "w", zipfile.ZIP_DEFLATED) as dst:
        for item in src.infolist():
            dst.writestr(item, src.read(item.filename))
        dst.write(dex, "classes.dex", zipfile.ZIP_DEFLATED)
        for abi in abis:
            lib = ROOT / "target" / "android-libs" / abi / LIB_NAME
            if not lib.exists():
                sys.exit(f"{lib} がありません（ビルドに失敗した？）")
            dst.write(lib, f"lib/{abi}/{LIB_NAME}", zipfile.ZIP_DEFLATED)
    aligned = work / "aligned.apk"
    run([tools / "zipalign.exe", "-f", "-p", "4", unaligned, aligned])
    OUT.mkdir(exist_ok=True)
    suffix = "-".join(ABI_NAMES[a] for a in abis)
    final = OUT / f"kumo-{VERSION}-android-{suffix}.apk"
    ensure_keystore()
    run([
        "java", "-jar", tools / "lib" / "apksigner.jar", "sign",
        "--ks", DEBUG_KEYSTORE, "--ks-pass", "pass:android", "--key-pass", "pass:android",
        "--out", final, aligned,
    ])
    run(["java", "-jar", tools / "lib" / "apksigner.jar", "verify", "--verbose", final])
    return final


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--abi", action="append", choices=list(ABI_NAMES), help="既定は arm64-v8a")
    parser.add_argument("--skip-build", action="store_true", help="Rust のビルドを省く")
    args = parser.parse_args()
    abis = args.abi or ["arm64-v8a"]
    home, ndk, tools, android_jar = sdk_paths()
    if not args.skip_build:
        build_libs(abis, ndk, home)
    final = package(abis, home, tools, android_jar)
    print(f"\n作りました: {final} ({final.stat().st_size / 1_000_000:.1f} MB)")


if __name__ == "__main__":
    main()
