package com.kumo.browser;

import android.app.Activity;
import android.content.Context;
import android.text.Editable;
import android.util.Log;
import android.text.InputType;
import android.text.Selection;
import android.view.KeyEvent;
import android.view.View;
import android.view.ViewGroup;
import android.view.WindowInsets;
import android.view.inputmethod.BaseInputConnection;
import android.view.inputmethod.EditorInfo;
import android.view.inputmethod.InputConnection;
import android.view.inputmethod.InputMethodManager;

/**
 * 日本語などの変換入力を受ける。NativeActivity は変換中の文字を扱えないので、
 * 文字入力用の見えない View を画面に足し、その InputConnection で受けた入力を Rust へ渡す。
 * Rust 側は kumo-android の Java_com_kumo_browser_ImeBridge_native*。
 */
public final class ImeBridge {
    private static final String TAG = "KumoIme";

    static {
        // NativeActivity は libkumo.so をクラスローダーとは別に読み込むので、native メソッドを
        // 見つけられるよう、このクラスのローダーにも結び付ける（読み込み済みなら同じものが返る）
        System.loadLibrary("kumo");
    }

    private static TextView view;

    private ImeBridge() {}

    private static native void nativePreedit(String text);

    private static native void nativeCommit(String text);

    private static native void nativeBackspace();

    private static native void nativeEnter();

    /** 画面の上端を覆うステータスバーなどの高さ（物理 px）。NativeActivity の描画面は画面全体なので、避けるのは描く側の仕事。 */
    public static int topInset(Activity activity) {
        try {
            WindowInsets insets = activity.getWindow().getDecorView().getRootWindowInsets();
            if (insets != null) {
                return Math.max(insets.getSystemWindowInsetTop(), 0);
            }
            int id = activity.getResources().getIdentifier("status_bar_height", "dimen", "android");
            return id > 0 ? activity.getResources().getDimensionPixelSize(id) : 0;
        } catch (Throwable e) {
            return 0;
        }
    }

    /** ソフトキーボードを出す・隠す。どのスレッドから呼んでもよい。 */
    public static void setActive(final Activity activity, final boolean active) {
        activity.runOnUiThread(
                new Runnable() {
                    @Override
                    public void run() {
                        try {
                            InputMethodManager imm =
                                    (InputMethodManager)
                                            activity.getSystemService(Context.INPUT_METHOD_SERVICE);
                            if (view == null) {
                                view = new TextView(activity);
                                activity.addContentView(view, new ViewGroup.LayoutParams(1, 1));
                            }
                            if (active) {
                                view.requestFocus();
                                imm.restartInput(view);
                                imm.showSoftInput(view, 0);
                            } else {
                                imm.hideSoftInputFromWindow(view.getWindowToken(), 0);
                            }
                        } catch (Throwable e) {
                            Log.e(TAG, "setActive failed", e);
                        }
                    }
                });
    }

    /** 文字入力の窓口になる、1px の見えない View。 */
    private static final class TextView extends View {
        TextView(Context context) {
            super(context);
            setFocusable(true);
            setFocusableInTouchMode(true);
        }

        @Override
        public boolean onCheckIsTextEditor() {
            return true;
        }

        @Override
        public InputConnection onCreateInputConnection(EditorInfo outAttrs) {
            // 変換や候補が効くよう、ふつうの文章の入力欄として振る舞う
            outAttrs.inputType = InputType.TYPE_CLASS_TEXT;
            outAttrs.imeOptions =
                    EditorInfo.IME_ACTION_GO
                            | EditorInfo.IME_FLAG_NO_FULLSCREEN
                            | EditorInfo.IME_FLAG_NO_EXTRACT_UI;
            return new Connection(this);
        }
    }

    /**
     * 入力欄の中身を `Editable` に写して持ち、変わるたびに Rust へ「消した数・確定した文字・変換中の文字」を送る。
     * キーボードは確定済みの単語を変換中に戻したりする（英語の BackSpace など）ので、
     * 確定済みの文字もこちらで覚えておかないと、Rust 側の文字と食い違う。
     */
    private static final class Connection extends BaseInputConnection {
        /** Rust へ送り済みの、確定した文字（変換中の部分を除く） */
        private String sent = "";

        /** 直前に確定済みの文字を消された（次の setComposingText が単語の変換し直しかもしれない） */
        private boolean justDeleted = false;

        Connection(View target) {
            super(target, true);
        }

        /** いまの `Editable` を、前回送った内容との差にして Rust へ送る。 */
        private void sync() {
            Editable editable = getEditable();
            String text = editable == null ? "" : editable.toString();
            int start = editable == null ? -1 : getComposingSpanStart(editable);
            int end = editable == null ? -1 : getComposingSpanEnd(editable);
            String composing = "";
            String committed = text;
            if (start >= 0 && end > start && end <= text.length()) {
                composing = text.substring(start, end);
                committed = text.substring(0, start) + text.substring(end);
            }
            int common = 0;
            int limit = Math.min(sent.length(), committed.length());
            while (common < limit && sent.charAt(common) == committed.charAt(common)) {
                common++;
            }
            // 書き込みの途中でサロゲートペアを割らない
            if (common > 0 && Character.isHighSurrogate(sent.charAt(common - 1))) {
                common--;
            }
            int removed = sent.codePointCount(common, sent.length());
            for (int i = 0; i < removed; i++) {
                nativeBackspace();
            }
            if (committed.length() > common) {
                nativeCommit(committed.substring(common));
            }
            sent = committed;
            nativePreedit(composing);
        }

        /**
         * 英語のキーボードは、確定した単語の後ろの空白を消すと、その単語を黙って変換中に戻し、
         * 続けて「その単語を1文字削ったもの」などで置き換えてくる。変換範囲が伝わってこないので、
         * 直前に文字を消されていて、新しい変換中の文字が直前の単語の続きに見えるときは、
         * その単語を変換範囲に戻しておく（置き換えが重複して残らないように）。
         */
        private void reopenWordIfEdited(CharSequence text) {
            Editable editable = getEditable();
            if (!justDeleted || editable == null || getComposingSpanStart(editable) >= 0) {
                return;
            }
            int cursor = Selection.getSelectionStart(editable);
            if (cursor <= 0) {
                return;
            }
            int wordStart = cursor;
            while (wordStart > 0 && isWordChar(editable.charAt(wordStart - 1))) {
                wordStart--;
            }
            if (wordStart == cursor) {
                return;
            }
            String kept = editable.subSequence(wordStart, cursor - 1).toString();
            if (text.toString().startsWith(kept)) {
                super.setComposingRegion(wordStart, cursor);
            }
        }

        private static boolean isWordChar(char c) {
            return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '\'';
        }

        private boolean hasText() {
            Editable editable = getEditable();
            return editable != null && editable.length() > 0;
        }

        private void reset() {
            Editable editable = getEditable();
            if (editable != null) {
                editable.clear();
            }
            sent = "";
        }

        @Override
        public boolean setComposingText(CharSequence text, int newCursorPosition) {
            reopenWordIfEdited(text);
            justDeleted = false;
            boolean ok = super.setComposingText(text, newCursorPosition);
            sync();
            return ok;
        }

        @Override
        public boolean setComposingRegion(int start, int end) {
            justDeleted = false;
            boolean ok = super.setComposingRegion(start, end);
            sync();
            return ok;
        }

        @Override
        public boolean commitText(CharSequence text, int newCursorPosition) {
            justDeleted = false;
            boolean ok = super.commitText(text, newCursorPosition);
            sync();
            return ok;
        }

        @Override
        public boolean finishComposingText() {
            // 変換を確定せずに閉じられたら、いま見えている文字をそのまま確定する
            boolean ok = super.finishComposingText();
            sync();
            return ok;
        }

        @Override
        public boolean deleteSurroundingText(int beforeLength, int afterLength) {
            if (!hasText()) {
                // 写しにない文字（Rust 側にだけある文字）は、1文字ずつ消す操作として渡す
                for (int i = 0; i < Math.max(beforeLength, 1); i++) {
                    nativeBackspace();
                }
                return true;
            }
            boolean ok = super.deleteSurroundingText(beforeLength, afterLength);
            justDeleted = true;
            sync();
            return ok;
        }

        @Override
        public boolean sendKeyEvent(KeyEvent event) {
            if (event.getAction() != KeyEvent.ACTION_DOWN) {
                return super.sendKeyEvent(event);
            }
            switch (event.getKeyCode()) {
                case KeyEvent.KEYCODE_DEL:
                    if (hasText()) {
                        Editable editable = getEditable();
                        int cursor = Math.max(Selection.getSelectionStart(editable), 0);
                        if (cursor > 0) {
                            int from = Character.offsetByCodePoints(editable, cursor, -1);
                            editable.delete(from, cursor);
                        }
                        sync();
                    } else {
                        nativeBackspace();
                    }
                    return true;
                case KeyEvent.KEYCODE_ENTER:
                    enter();
                    return true;
                default:
                    return super.sendKeyEvent(event);
            }
        }

        @Override
        public boolean performEditorAction(int editorAction) {
            enter();
            return true;
        }

        /** 変換中の文字を確定してから Enter（実行）。入力欄は空に戻る。 */
        private void enter() {
            super.finishComposingText();
            sync();
            nativeEnter();
            reset();
        }
    }
}
