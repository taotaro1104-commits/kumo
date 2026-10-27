//! Kumo のプロセス間通信の土台：共有メモリ、リングの領域管理、ローカルソケット。
//! 共有メモリのマップに `unsafe` が要るため、kumo-mem と並んで unsafe を許す数少ない crate。

mod ring;
mod shm;

pub use ring::RingAllocator;
pub use shm::{SharedMemory, SharedView, is_shared_path, random_token, unique_path};

use std::io::{self, Read, Write};
use std::sync::Arc;

use interprocess::local_socket::prelude::*;
use interprocess::local_socket::{GenericNamespaced, ListenerOptions};

pub use interprocess::local_socket::{Listener, Stream};

/// ローカルソケット名を作る（Windows は名前付きパイプ、Unix はソケット）。
/// 他のプロセスに先に同じ名前で待ち受けられないよう、推測しにくい値を含める。
pub fn socket_name(prefix: &str) -> String {
    format!(
        "kumo-{prefix}-{}-{:016x}.sock",
        std::process::id(),
        random_token()
    )
}

/// ローカルソケットで待ち受ける。
pub fn listen(name: &str) -> io::Result<Listener> {
    let name = name.to_ns_name::<GenericNamespaced>()?;
    ListenerOptions::new().name(name).create_sync()
}

/// 次の接続を待って受け付ける。
pub fn accept(listener: &Listener) -> io::Result<Connection> {
    Ok(Connection::new(listener.accept()?))
}

/// 接続。複製して受信用スレッドと送信用に分けて使う（`&Stream` は読み書きできる）。
#[derive(Clone)]
pub struct Connection(Arc<Stream>);

impl Connection {
    pub fn new(stream: Stream) -> Self {
        Self(Arc::new(stream))
    }
}

impl Read for Connection {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        (&*self.0).read(buf)
    }
}

impl Write for Connection {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        (&*self.0).write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        (&*self.0).flush()
    }
}

/// ローカルソケットに接続する。
pub fn connect(name: &str) -> io::Result<Connection> {
    let name = name.to_ns_name::<GenericNamespaced>()?;
    Ok(Connection::new(Stream::connect(name)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_socket_round_trip() {
        let name = format!("kumo-test-{}-{}.sock", std::process::id(), line!());
        let listener = listen(&name).unwrap();
        let server = std::thread::spawn(move || {
            let mut conn = Connection::new(listener.accept().unwrap());
            let mut buf = [0u8; 4];
            conn.read_exact(&mut buf).unwrap();
            conn.write_all(&buf.map(|b| b + 1)).unwrap();
        });
        let conn = connect(&name).unwrap();
        let (mut rx, mut tx) = (conn.clone(), conn);
        tx.write_all(&[1, 2, 3, 4]).unwrap();
        let mut reply = [0u8; 4];
        rx.read_exact(&mut reply).unwrap();
        assert_eq!(reply, [2, 3, 4, 5]);
        server.join().unwrap();
    }
}
