use std::path::PathBuf;
use std::sync::Arc;

use kumo_content::ContentEngine;
use kumo_content::protocol::FromContent;
use kumo_net::service::{NetworkConfig, NetworkService};
use url::Url;

/// テスト用のローカルファイルのサイト。
pub struct Site {
    pub dir: PathBuf,
}

impl Site {
    pub fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("kumo-content-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let write = |file: &str, body: &str| std::fs::write(dir.join(file), body).unwrap();
        write(
            "red.html",
            "<html><head><title>Red</title><link rel=stylesheet href=red.css></head><body></body></html>",
        );
        write(
            "red.css",
            "html, body { margin: 0; background: #ff0000; height: 1000px }",
        );
        write(
            "long.html",
            "<html><head><title>Long</title></head><body style='margin:0;font-size:20px'>\
             <a href=red.html>red</a><div style='height:1980px'></div>\
             <h2 id=far style='margin:0'>Far</h2><div style='height:2000px'></div></body></html>",
        );
        write(
            "form.html",
            "<html><head><title>Form</title></head><body style='margin:0'>\
             <form action='result.html'><input name=q placeholder='Search'>\
             <input type=checkbox name=all value=yes><input type=submit value=Go></form></body></html>",
        );
        write("result.html", "<title>Result</title><p>ok</p>");
        Self { dir }
    }

    pub fn url(&self, file: &str) -> Url {
        let (path, fragment) = file
            .split_once('#')
            .map_or((file, None), |(p, f)| (p, Some(f)));
        let mut url = Url::from_file_path(self.dir.join(path)).unwrap();
        url.set_fragment(fragment);
        url
    }
}

impl Drop for Site {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[allow(dead_code)]
pub fn engine() -> ContentEngine {
    ContentEngine::new(
        Arc::new(NetworkService::new(&NetworkConfig::default())),
        || 0,
    )
}

/// 取得の依頼をその場で（同じスレッドで）済ませ、描画して通知を取り出す。
/// `fetch` が `false` なら取得はせずに描画だけする。
#[allow(dead_code)]
pub fn pump(engine: &mut ContentEngine, fetch: bool) -> Vec<FromContent> {
    let fetcher = engine.fetcher();
    let mut held = Vec::new();
    loop {
        let jobs = engine.take_jobs();
        if jobs.is_empty() {
            break;
        }
        for job in jobs {
            if fetch {
                if let Some(done) = job.run(&*fetcher) {
                    engine.done(done);
                }
            } else {
                held.push(job);
            }
        }
        if !fetch {
            break;
        }
    }
    engine.render();
    let out = engine.take_output();
    // 取得しなかった依頼は次の pump で処理できるよう戻す
    for job in held {
        engine.requeue(job);
    }
    out
}
