//! 鍵の提供元の登録（プロセスで1回だけなので、他のテストと分けて1つのテストにまとめる）。

use kumo_browser::{Vault, set_key_provider};

fn provider(create: bool) -> Option<[u8; 32]> {
    create.then_some([7u8; 32])
}

#[test]
fn registered_provider_supplies_the_key() {
    set_key_provider(provider);
    assert!(
        Vault::system(false).is_none(),
        "作らない指定で鍵がなければ None"
    );
    let vault = Vault::system(true).expect("提供元の鍵");
    let sealed = vault.seal(b"hunter2", b"password").unwrap();
    let again = Vault::from_key([7u8; 32]);
    assert_eq!(again.open(&sealed, b"password").unwrap(), b"hunter2");
}
