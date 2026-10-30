//! Java（`android/java` の `ImeBridge`・`KeyVault`）との橋渡し。
//!
//! 変換入力は Java の InputConnection → `Java_..._native*`（UI スレッド）→ [`kumo_viewer::ime::push`]。
//! キーボードの表示と Keystore の鍵は、Rust から Java の静的メソッドを呼ぶ。

use std::sync::atomic::{AtomicUsize, Ordering};

use jni::errors::Result as JniResult;
use jni::objects::{JByteArray, JClass, JObject, JString, JValue};
use jni::{JNIEnv, JavaVM};
use kumo_browser::VAULT_KEY_LEN;
use kumo_viewer::ime::{ImeInput, push};

/// 実行中のアクティビティの JNI 参照（`AndroidApp::activity_as_ptr`）。`AndroidApp` が生きている間は有効。
static ACTIVITY: AtomicUsize = AtomicUsize::new(0);

/// `android_main` の最初に、アクティビティを登録する。
pub fn set_activity(activity: *mut std::ffi::c_void) {
    ACTIVITY.store(activity as usize, Ordering::SeqCst);
}

/// 現在のスレッドを JVM につなぎ、`f` へ渡す（Application の Context と、登録済みのアクティビティ）。
/// Java の例外が残ったときは内容を出して消し、`None` を返す。
fn with_java<R>(f: impl FnOnce(&mut JNIEnv, &JObject, &JObject) -> JniResult<R>) -> Option<R> {
    let context = ndk_context::android_context();
    // SAFETY: android-activity が設定した、アプリが動いている間は有効な JavaVM のポインタ。
    let vm = unsafe { JavaVM::from_raw(context.vm().cast()) }.ok()?;
    let mut guard = vm.attach_current_thread().ok()?;
    // SAFETY: android-activity が設定した Application のグローバル参照。
    // `JObject` は参照を解放しないので、借りるだけ。
    let application = unsafe { JObject::from_raw(context.context().cast()) };
    // SAFETY: `set_activity` で登録された、`AndroidApp` が生きている間は有効なグローバル参照（未登録なら null）。
    let activity =
        unsafe { JObject::from_raw(ACTIVITY.load(Ordering::SeqCst) as jni::sys::jobject) };
    let env: &mut JNIEnv = &mut guard;
    let result = f(env, &application, &activity);
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_describe();
        let _ = env.exception_clear();
        return None;
    }
    result.ok()
}

/// アプリの ClassLoader から Java のクラスを得る（ネイティブのスレッドの `FindClass` はアプリのクラスを見つけられない）。
fn load_class<'a>(env: &mut JNIEnv<'a>, activity: &JObject, name: &str) -> JniResult<JClass<'a>> {
    let loader = env
        .call_method(activity, "getClassLoader", "()Ljava/lang/ClassLoader;", &[])?
        .l()?;
    let name = env.new_string(name)?;
    let class = env
        .call_method(
            &loader,
            "loadClass",
            "(Ljava/lang/String;)Ljava/lang/Class;",
            &[JValue::Object(&name)],
        )?
        .l()?;
    Ok(JClass::from(class))
}

/// ソフトキーボードを出す・隠す。
pub fn show_keyboard(show: bool) {
    with_java(|env, application, activity| {
        if activity.is_null() {
            return Ok(());
        }
        let class = load_class(env, application, "com.kumo.browser.ImeBridge")?;
        env.call_static_method(
            class,
            "setActive",
            "(Landroid/app/Activity;Z)V",
            &[JValue::Object(activity), JValue::Bool(show.into())],
        )?;
        Ok(())
    });
}

/// 画面の上端を覆うステータスバーの高さ（物理 px）。取れなければ 0。
pub fn top_inset() -> u32 {
    with_java(|env, application, activity| {
        if activity.is_null() {
            return Ok(0);
        }
        let class = load_class(env, application, "com.kumo.browser.ImeBridge")?;
        let px = env
            .call_static_method(
                class,
                "topInset",
                "(Landroid/app/Activity;)I",
                &[JValue::Object(activity)],
            )?
            .i()?;
        Ok(px.max(0) as u32)
    })
    .unwrap_or(0)
}

/// 保管庫の鍵を Android Keystore から得る（`create` なら、なければ作る）。
pub fn vault_key(create: bool) -> Option<[u8; VAULT_KEY_LEN]> {
    with_java(|env, application, _activity| {
        let class = load_class(env, application, "com.kumo.browser.KeyVault")?;
        let bytes = env
            .call_static_method(
                class,
                "loadKey",
                "(Landroid/content/Context;Z)[B",
                &[JValue::Object(application), JValue::Bool(create.into())],
            )?
            .l()?;
        if bytes.is_null() {
            return Ok(None);
        }
        let bytes = env.convert_byte_array(JByteArray::from(bytes))?;
        Ok(<[u8; VAULT_KEY_LEN]>::try_from(bytes.as_slice()).ok())
    })
    .flatten()
}

fn string_of(env: &mut JNIEnv, text: &JString) -> String {
    env.get_string(text).map(String::from).unwrap_or_default()
}

// 以下は Java の `static native` メソッドの実体。JNI が名前で探すので、名前と型は Java 側と一致させる。

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_kumo_browser_ImeBridge_nativePreedit(
    mut env: JNIEnv,
    _class: JClass,
    text: JString,
) {
    push(ImeInput::Preedit(string_of(&mut env, &text)));
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_kumo_browser_ImeBridge_nativeCommit(
    mut env: JNIEnv,
    _class: JClass,
    text: JString,
) {
    push(ImeInput::Commit(string_of(&mut env, &text)));
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_kumo_browser_ImeBridge_nativeBackspace(
    _env: JNIEnv,
    _class: JClass,
) {
    push(ImeInput::Backspace);
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_kumo_browser_ImeBridge_nativeEnter(_env: JNIEnv, _class: JClass) {
    push(ImeInput::Enter);
}
