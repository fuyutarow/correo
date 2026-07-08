//! ja-slop-lint CLI — 薄い entry。日本語散文の質を stdin/file で検出する。
//! 用法:
//!   `ja-slop-lint codemix [--threshold N] [--exempt ALLOW.md] [FILE...]`  ルー語密度（file 無し=stdin）
//!   `ja-slop-lint coinage [--dict-dir D] [--allow A.md]... [--diff] [--strict] [--advisory] [FILE...]`  造語（辞書 membership・要 --features coinage）
//! hook はこの binary へ薄く委譲する（検出ロジックは持たない）。calque は順次追加。
use std::path::PathBuf;
use std::process::exit;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut iter = args.iter();
    match iter.next().map(String::as_str) {
        Some("--version" | "-V") => {
            println!("ja-slop-lint {}", env!("CARGO_PKG_VERSION"));
            exit(0);
        }
        Some("codemix") => {
            let mut threshold = 8.0f64;
            let mut exempt_md: Option<PathBuf> = None;
            let mut files: Vec<String> = Vec::new();
            while let Some(a) = iter.next() {
                match a.as_str() {
                    "--threshold" => {
                        threshold = iter
                            .next()
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(threshold);
                    }
                    "--exempt" => {
                        exempt_md = iter.next().map(PathBuf::from);
                    }
                    f => files.push(f.to_string()),
                }
            }
            // exempt の allow-list は明示注入（呼び出し側が注入）。未指定＝空（識別子/ALLCAPS の
            // built-in 除外のみ効く）。
            let exempt = exempt_md
                .map(|p| ja_slop_lint::codemix::domain_vocab(&p))
                .unwrap_or_default();
            exit(ja_slop_lint::codemix::codemix(threshold, &files, &exempt));
        }
        Some("coinage") => {
            let mut a = ja_slop_lint::coinage::CoinageArgs {
                dict_dir: None,
                allow: Vec::new(),
                diff: false,
                strict: false,
                advisory: false,
                files: Vec::new(),
            };
            while let Some(arg) = iter.next() {
                match arg.as_str() {
                    "--dict-dir" => a.dict_dir = iter.next().map(PathBuf::from),
                    "--allow" => {
                        if let Some(s) = iter.next() {
                            a.allow.push(s.to_string());
                        }
                    }
                    "--diff" => a.diff = true,
                    "--strict" => a.strict = true,
                    "--advisory" => a.advisory = true,
                    f => a.files.push(f.to_string()),
                }
            }
            match ja_slop_lint::coinage::run_coinage(a) {
                Ok(code) => exit(code),
                Err(e) => {
                    eprintln!("ja-slop-lint coinage: {e}");
                    exit(2);
                }
            }
        }
        _ => {
            eprintln!(
                "用法: ja-slop-lint <codemix|coinage> ...（codemix=ルー語密度・coinage=造語）"
            );
            exit(2);
        }
    }
}
