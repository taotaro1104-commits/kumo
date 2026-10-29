//! テスト用の小さな HTTP/1.1 サーバ。パスごとに決まった応答を返し、到達回数を数える。

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

pub struct TestServer {
    pub base: String,
    hits: Arc<Mutex<HashMap<String, usize>>>,
}

impl TestServer {
    pub fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let hits: Arc<Mutex<HashMap<String, usize>>> = Arc::default();
        let counter = hits.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                let counter = counter.clone();
                std::thread::spawn(move || serve(stream, &counter));
            }
        });
        Self { base, hits }
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base)
    }

    #[allow(dead_code)]
    pub fn hits(&self, path: &str) -> usize {
        self.hits.lock().unwrap().get(path).copied().unwrap_or(0)
    }
}

fn serve(mut stream: std::net::TcpStream, counter: &Mutex<HashMap<String, usize>>) {
    let Ok(clone) = stream.try_clone() else {
        return;
    };
    let mut reader = BufReader::new(clone);
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() {
        return;
    }
    let path = request_line
        .split_whitespace()
        .nth(1)
        .unwrap_or("/")
        .to_owned();
    let method = request_line
        .split_whitespace()
        .next()
        .unwrap_or("GET")
        .to_owned();
    let mut if_none_match = None;
    let mut cookie = String::new();
    let mut content_length = 0usize;
    let mut content_type = String::new();
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
            break;
        }
        let lower = line.to_ascii_lowercase();
        if let Some(v) = lower.strip_prefix("if-none-match:") {
            if_none_match = Some(v.trim().to_owned());
        }
        if lower.starts_with("cookie:") {
            cookie = line[7..].trim().to_owned();
        }
        if let Some(v) = lower.strip_prefix("content-length:") {
            content_length = v.trim().parse().unwrap_or(0);
        }
        if let Some(v) = lower.strip_prefix("content-type:") {
            content_type = v.trim().to_owned();
        }
    }
    let mut body = vec![0u8; content_length.min(1 << 20)];
    let _ = std::io::Read::read_exact(&mut reader, &mut body);
    *counter.lock().unwrap().entry(path.clone()).or_default() += 1;
    let echo = format!("{method} {content_type} {}", String::from_utf8_lossy(&body));
    let response = match path.split('?').next().unwrap_or("") {
        "/echo-cookie" => ok("", "text/plain", &cookie),
        "/echo" => ok(
            "",
            "text/html",
            &format!("<title>{echo}</title><p>{}</p>", path),
        ),
        "/file.zip" => ok(
            "Content-Disposition: attachment; filename=\"my file.zip\"\r\n",
            "application/zip",
            "PKzipbody",
        ),
        "/data.bin" => ok("", "application/octet-stream", "0123456789"),
        "/cors" => ok("Access-Control-Allow-Origin: *\r\n", "text/plain", "public"),
        "/big" => ok(
            "Cache-Control: no-store\r\n",
            "text/plain",
            &big_body(&path),
        ),
        _ => respond(&path, if_none_match.as_deref()),
    };
    let _ = stream.write_all(response.as_bytes());
}

/// `/big?n=バイト数&c=文字` の本文：文字 c を n 個並べる。
pub fn big_body(path: &str) -> String {
    let query = path.split_once('?').map_or("", |(_, q)| q);
    let mut n = 0usize;
    let mut c = "x";
    for pair in query.split('&') {
        match pair.split_once('=') {
            Some(("n", v)) => n = v.parse().unwrap_or(0),
            Some(("c", v)) => c = v,
            _ => {}
        }
    }
    c.repeat(n)
}

fn ok(headers: &str, content_type: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n{headers}\r\n{body}",
        body.len()
    )
}

fn respond(path: &str, if_none_match: Option<&str>) -> String {
    const NOT_MODIFIED: &str = "HTTP/1.1 304 Not Modified\r\nETag: \"v1\"\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n";
    match path {
        "/fresh.css" => ok("Cache-Control: max-age=600\r\n", "text/css", "a{color:red}"),
        "/etag.css" if if_none_match == Some("\"v1\"") => NOT_MODIFIED.to_owned(),
        "/etag.css" => ok("ETag: \"v1\"\r\nCache-Control: no-cache\r\n", "text/css", "b{color:blue}"),
        "/nostore.css" => ok("Cache-Control: no-store\r\n", "text/css", "c{}"),
        "/page.html" => ok(
            "Cache-Control: max-age=600\r\n",
            "text/html",
            "<html><head><link rel=stylesheet href=fresh.css></head><body><p>Hello <a href=next.html>next</a></p></body></html>",
        ),
        "/next.html" => ok("", "text/html", "<p>Next page</p>"),
        "/set-cookie" => ok(
            "Set-Cookie: persistent=yes; Max-Age=3600; Path=/\r\nSet-Cookie: session=1; Path=/\r\n",
            "text/plain",
            "set",
        ),
        "/loop" => redirect("302 Found", "/loop"),
        "/redirect-chain" => redirect("301 Moved", "/redirect"),
        "/redirect-to-file" => redirect("302 Found", "file:///etc/passwd"),
        "/redirect" => {
            "HTTP/1.1 302 Found\r\nLocation: /fresh.css\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                .to_owned()
        }
        _ => "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_owned(),
    }
}

fn redirect(status: &str, location: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    )
}
