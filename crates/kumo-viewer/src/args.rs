//! コマンドライン引数。

use std::path::PathBuf;

#[derive(Debug)]
pub struct Args {
    pub targets: Vec<String>,
    pub width: u32,
    pub height: u32,
    pub scroll: f32,
    /// 拡大率（%）。ヘッドレスの描画で使う
    pub zoom: u32,
    pub headless: Option<String>,
    pub mem_report: bool,
    pub in_process: bool,
    pub no_js: bool,
    /// 画像を読み込まない（保存はしない）
    pub no_images: bool,
    /// 読み込みの負荷レベル（1〜5）。指定すると保存済みの設定より優先する（保存はしない）
    pub load_level: Option<kumo_browser::LoadLevel>,
    /// 表示言語（ja・en・zh・ko・es）。指定すると保存済みの設定より優先する（保存はしない）
    pub lang: Option<kumo_browser::Lang>,
    pub no_disk_cache: bool,
    /// ヘッドレスで、読み込み後にタイマーを動かしながら待つ時間（ms）
    pub wait_ms: u64,
    /// ヘッドレスで、待った後にページで評価する JS の式（結果を標準出力へ）
    pub eval: Vec<String>,
    /// メモリの予算（MB）。超えたら背景のタブを降格する
    pub memory_budget_mb: Option<u64>,
    /// 背景に回って凍結するまで・見なくなって破棄するまでの秒数
    pub freeze_after_secs: Option<u64>,
    pub discard_after_secs: Option<u64>,
    pub filter_lists: Vec<PathBuf>,
}

pub const USAGE: &str = "usage: kumo-viewer [URL|FILE ...] [--width N] [--height N] [--scroll Y] [--zoom PERCENT] \
[--headless out.png] [--mem-report] [--in-process] [--no-js] [--no-images] [--load-level 1-5] [--lang ja|en|zh|ko|es] [--no-disk-cache] [--wait MS] [--eval JS] [--memory-budget MB] [--freeze-after SEC] [--discard-after SEC] [--filter-list FILE ...]";

pub fn parse(raw: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut args = Args {
        targets: Vec::new(),
        width: 1280,
        height: 800,
        scroll: 0.0,
        zoom: 100,
        headless: None,
        mem_report: false,
        in_process: false,
        no_js: false,
        no_images: false,
        load_level: None,
        lang: None,
        no_disk_cache: false,
        wait_ms: 0,
        eval: Vec::new(),
        memory_budget_mb: None,
        freeze_after_secs: None,
        discard_after_secs: None,
        filter_lists: Vec::new(),
    };
    let mut raw = raw.peekable();
    while let Some(arg) = raw.next() {
        let mut value = |name: &str| raw.next().ok_or_else(|| format!("{name} には値が必要です"));
        match arg.as_str() {
            "--width" => args.width = parse_dim(&value("--width")?)?,
            "--height" => args.height = parse_dim(&value("--height")?)?,
            "--scroll" => {
                args.scroll = value("--scroll")?
                    .parse()
                    .map_err(|_| "--scroll は数値".to_owned())?
            }
            "--zoom" => {
                args.zoom = value("--zoom")?
                    .parse()
                    .map_err(|_| "--zoom は 25〜500 の整数（%）".to_owned())?
            }
            "--headless" => args.headless = Some(value("--headless")?),
            "--filter-list" => args.filter_lists.push(value("--filter-list")?.into()),
            "--memory-budget" => {
                args.memory_budget_mb =
                    Some(parse_count("--memory-budget", &value("--memory-budget")?)?)
            }
            "--freeze-after" => {
                args.freeze_after_secs =
                    Some(parse_count("--freeze-after", &value("--freeze-after")?)?)
            }
            "--discard-after" => {
                args.discard_after_secs =
                    Some(parse_count("--discard-after", &value("--discard-after")?)?)
            }
            "--eval" => args.eval.push(value("--eval")?),
            "--wait" => args.wait_ms = parse_count("--wait", &value("--wait")?)?,
            "--mem-report" => args.mem_report = true,
            "--in-process" => args.in_process = true,
            "--no-js" => args.no_js = true,
            "--no-images" => args.no_images = true,
            "--load-level" => {
                let n = value("--load-level")?;
                args.load_level = Some(
                    n.parse::<u8>()
                        .ok()
                        .and_then(kumo_browser::LoadLevel::from_number)
                        .ok_or_else(|| format!("--load-level は 1〜5: {n}"))?,
                );
            }
            "--lang" => {
                let code = value("--lang")?;
                args.lang = Some(
                    kumo_browser::Lang::from_code(&code)
                        .ok_or_else(|| format!("--lang は ja・en・zh・ko・es: {code}"))?,
                );
            }
            "--no-disk-cache" => args.no_disk_cache = true,
            "-h" | "--help" => return Err(USAGE.to_owned()),
            other if other.starts_with("--") => {
                return Err(format!("不明なオプション: {other}\n{USAGE}"));
            }
            other => args.targets.push(other.to_owned()),
        }
    }
    Ok(args)
}

fn parse_count(name: &str, s: &str) -> Result<u64, String> {
    match s.parse::<u64>() {
        Ok(v) if v > 0 => Ok(v),
        _ => Err(format!("{name} は 1 以上の整数: {s}")),
    }
}

fn parse_dim(s: &str) -> Result<u32, String> {
    match s.parse::<u32>() {
        Ok(v) if (100..=8192).contains(&v) => Ok(v),
        _ => Err(format!("幅・高さは 100〜8192 の整数: {s}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Result<Args, String> {
        parse(list.iter().map(|s| s.to_string()))
    }

    #[test]
    fn lang_takes_a_language_code() {
        assert_eq!(
            args(&["--lang", "en"]).unwrap().lang,
            Some(kumo_browser::Lang::En)
        );
        assert!(args(&[]).unwrap().lang.is_none());
        assert!(args(&["--lang", "fr"]).is_err());
    }

    #[test]
    fn load_level_takes_one_to_five() {
        let a = args(&["--load-level", "5"]).unwrap();
        assert_eq!(a.load_level, Some(kumo_browser::LoadLevel::Maximum));
        assert!(args(&[]).unwrap().load_level.is_none());
        assert!(args(&["--load-level", "0"]).is_err());
        assert!(args(&["--load-level", "6"]).is_err());
        assert!(args(&["--load-level", "x"]).is_err());
    }

    #[test]
    fn parses_options_and_multiple_targets() {
        let a = args(&[
            "a.html",
            "https://b/",
            "--width",
            "1920",
            "--headless",
            "o.png",
            "--mem-report",
            "--in-process",
        ])
        .unwrap();
        assert_eq!(a.targets, vec!["a.html", "https://b/"]);
        assert_eq!((a.width, a.height), (1920, 800));
        assert_eq!(a.headless.as_deref(), Some("o.png"));
        assert!(a.mem_report && a.in_process && !a.no_disk_cache);
        assert!(
            args(&[]).unwrap().targets.is_empty(),
            "URL なしでも起動できる"
        );
    }

    #[test]
    fn rejects_bad_input() {
        assert!(args(&["x", "--width", "5"]).is_err());
        assert!(args(&["x", "--bogus"]).is_err());
        assert!(args(&["x", "--width"]).is_err());
        assert!(args(&["--scroll", "abc"]).is_err());
        assert!(args(&["--memory-budget", "0"]).is_err());
    }

    #[test]
    fn parses_memory_policy_options() {
        let a = args(&[
            "--memory-budget",
            "150",
            "--freeze-after",
            "30",
            "--discard-after",
            "300",
        ])
        .unwrap();
        assert_eq!(a.memory_budget_mb, Some(150));
        assert_eq!(
            (a.freeze_after_secs, a.discard_after_secs),
            (Some(30), Some(300))
        );
        assert_eq!(args(&[]).unwrap().memory_budget_mb, None);
    }
}
