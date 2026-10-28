//! コマンドライン引数。

use std::path::PathBuf;

#[derive(Debug)]
pub struct Args {
    pub targets: Vec<String>,
    pub width: u32,
    pub height: u32,
    pub scroll: f32,
    pub headless: Option<String>,
    pub mem_report: bool,
    pub in_process: bool,
    pub no_js: bool,
    pub no_disk_cache: bool,
    pub filter_lists: Vec<PathBuf>,
}

pub const USAGE: &str = "usage: kumo-viewer [URL|FILE ...] [--width N] [--height N] [--scroll Y] \
[--headless out.png] [--mem-report] [--in-process] [--no-js] [--no-disk-cache] [--filter-list FILE ...]";

pub fn parse(raw: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut args = Args {
        targets: Vec::new(),
        width: 1280,
        height: 800,
        scroll: 0.0,
        headless: None,
        mem_report: false,
        in_process: false,
        no_js: false,
        no_disk_cache: false,
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
            "--headless" => args.headless = Some(value("--headless")?),
            "--filter-list" => args.filter_lists.push(value("--filter-list")?.into()),
            "--mem-report" => args.mem_report = true,
            "--in-process" => args.in_process = true,
            "--no-js" => args.no_js = true,
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
    }
}
