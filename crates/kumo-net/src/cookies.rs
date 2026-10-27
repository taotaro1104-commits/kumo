//! Cookie の永続化。期限つき（永続）Cookie だけを JSON で保存し、次回起動時に読み込む。
//! セッション Cookie（期限なし）はブラウザを閉じたら消える（一般的なブラウザと同じ扱い）。

use std::io::BufReader;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// 取得のたびに保存すると重いので、この間隔より頻繁には書かない。
const SAVE_INTERVAL: Duration = Duration::from_secs(30);

pub struct CookiePersistence {
    path: Option<PathBuf>,
    last_save: Mutex<Instant>,
}

impl CookiePersistence {
    /// `path` が `None` なら保存しない（メモリ上だけ）。
    pub fn new(path: Option<PathBuf>) -> Self {
        Self {
            path,
            last_save: Mutex::new(Instant::now()),
        }
    }

    /// 保存済みの Cookie をエージェントに読み込む。ファイルがなければ何もしない。
    pub fn load(&self, agent: &ureq::Agent) {
        let Some(path) = &self.path else { return };
        let Ok(file) = std::fs::File::open(path) else {
            return;
        };
        if let Err(e) = agent.cookie_jar_lock().load_json(BufReader::new(file)) {
            eprintln!("[cookie] {} を読めません: {e}", path.display());
        }
    }

    /// すぐに保存する（終了時）。一時ファイルに書いてから置き換えるので途中で落ちても壊れない。
    pub fn save(&self, agent: &ureq::Agent) {
        let Some(path) = &self.path else { return };
        let mut json = Vec::new();
        if let Err(e) = agent.cookie_jar_lock().save_json(&mut json) {
            eprintln!("[cookie] 書き出せません: {e}");
            return;
        }
        let tmp = path.with_extension("json.tmp");
        let result = std::fs::write(&tmp, &json).and_then(|()| std::fs::rename(&tmp, path));
        if let Err(e) = result {
            eprintln!("[cookie] {} に保存できません: {e}", path.display());
        }
        if let Ok(mut last) = self.last_save.lock() {
            *last = Instant::now();
        }
    }

    /// 前回の保存から一定時間たっていれば保存する（取得のたびに呼ぶ）。
    pub fn save_if_due(&self, agent: &ureq::Agent) {
        let due = self
            .last_save
            .lock()
            .is_ok_and(|last| last.elapsed() >= SAVE_INTERVAL);
        if due {
            self.save(agent);
        }
    }
}
