package com.kumo.browser;

import android.content.Context;
import android.security.keystore.KeyGenParameterSpec;
import android.security.keystore.KeyProperties;

import java.io.File;
import java.io.FileOutputStream;
import java.nio.file.Files;
import java.security.KeyStore;
import java.security.SecureRandom;

import javax.crypto.Cipher;
import javax.crypto.KeyGenerator;
import javax.crypto.SecretKey;
import javax.crypto.spec.GCMParameterSpec;

/**
 * パスワード保管庫の鍵（32バイト）を預かる。Rust から JNI で呼ばれる。
 *
 * 鍵そのものは Android Keystore の AES 鍵で包んでアプリ内のファイルに置く。
 * Keystore の鍵は端末の外へ出せないので、ファイルだけが持ち出されても保管庫は開けない。
 */
public final class KeyVault {
    private static final String ALIAS = "kumo-vault-wrap";
    private static final String FILE = "vault.key";
    private static final int KEY_LEN = 32;
    private static final int IV_LEN = 12;

    private KeyVault() {}

    /**
     * 保管庫の鍵を返す。まだ無ければ、create が true のときだけ作る。使えない環境では null。
     */
    public static byte[] loadKey(Context context, boolean create) {
        try {
            KeyStore store = KeyStore.getInstance("AndroidKeyStore");
            store.load(null);
            File file = new File(context.getFilesDir(), FILE);
            if (file.exists() && store.containsAlias(ALIAS)) {
                byte[] key = unwrap(store, Files.readAllBytes(file.toPath()));
                if (key != null) {
                    return key;
                }
            }
            if (!create) {
                return null;
            }
            // 無い（または開けない）ので作り直す。以前の鍵で暗号化したものは読めなくなる
            SecretKey wrapper = newWrapperKey();
            byte[] key = new byte[KEY_LEN];
            new SecureRandom().nextBytes(key);
            Cipher cipher = Cipher.getInstance("AES/GCM/NoPadding");
            cipher.init(Cipher.ENCRYPT_MODE, wrapper);
            byte[] sealed = cipher.doFinal(key);
            byte[] iv = cipher.getIV();
            try (FileOutputStream out = new FileOutputStream(file)) {
                out.write(iv);
                out.write(sealed);
            }
            return key;
        } catch (Exception e) {
            return null;
        }
    }

    private static byte[] unwrap(KeyStore store, byte[] blob) {
        try {
            if (blob.length <= IV_LEN) {
                return null;
            }
            SecretKey wrapper = (SecretKey) store.getKey(ALIAS, null);
            Cipher cipher = Cipher.getInstance("AES/GCM/NoPadding");
            cipher.init(Cipher.DECRYPT_MODE, wrapper, new GCMParameterSpec(128, blob, 0, IV_LEN));
            byte[] key = cipher.doFinal(blob, IV_LEN, blob.length - IV_LEN);
            return key.length == KEY_LEN ? key : null;
        } catch (Exception e) {
            return null;
        }
    }

    private static SecretKey newWrapperKey() throws Exception {
        KeyGenerator generator =
                KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore");
        generator.init(
                new KeyGenParameterSpec.Builder(
                                ALIAS, KeyProperties.PURPOSE_ENCRYPT | KeyProperties.PURPOSE_DECRYPT)
                        .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                        .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                        .setKeySize(256)
                        .build());
        return generator.generateKey();
    }
}
