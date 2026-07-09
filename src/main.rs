//! correo CLI — 薄い entry。日本語散文の質を stdin/file で検出する。
//! 検出ロジックは lib（codemix / coinage）が持ち、この binary は薄く委譲するだけ。calque は順次追加。
//! 引数解釈は clap derive（旧・手書き iter parse を置換 — `--flag=value`・`--`・未知フラグ error・
//! `--help` を得る）。hook はこの binary へ薄く委譲する。
use std::path::PathBuf;
use std::process::exit;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Clone, Copy, PartialEq, ValueEnum)]
enum Format {
    Text,
    Json,
}

#[derive(Parser)]
#[command(
    name = "correo",
    version,
    about = "日本語 LLM slop（言語の不自然さ）を検出する lint"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// ルー語密度（地の文の latin / 100字）。file 無し=stdin。
    Codemix {
        /// 密度閾値（latin / 100 JA字）
        #[arg(long, default_value_t = 8.0)]
        threshold: f64,
        /// allow-list（除外語彙・統制語彙 registry の .md・複数可。coinage と同名 flag に統一）
        #[arg(long)]
        allow: Vec<PathBuf>,
        /// 対象 file（無指定=stdin）
        files: Vec<String>,
    },
    /// 全検出器を一括実行（引数ゼロで cwd 以下の *.md を走査・correo.toml 自動発見・inline 抑制対応）。
    Check {
        /// codemix の密度閾値（latin / 100 JA字。既定 8・correo.toml で設定可）
        #[arg(long)]
        threshold: Option<f64>,
        /// allow-list registry file の追加注入（correo.toml の allow が主経路・これは補助）
        #[arg(long)]
        allow: Vec<PathBuf>,
        /// kinoshita: 一文の最大文字数（既定 100・correo.toml で設定可）
        #[arg(long)]
        max_sentence: Option<usize>,
        /// kinoshita: 一文の最大読点数（既定 4・correo.toml で設定可）
        #[arg(long)]
        max_ten: Option<usize>,
        /// 出力形式（text=人向け / json=judge 連携・機械可読）
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
        /// 対象 file（省略時: cwd 以下の *.md を .gitignore 準拠で全走査）
        files: Vec<String>,
    },
    /// 木下是雄 HARD 層（文長・読点過多・文体混在・慣用二重否定・ぼかし連発・指示語連鎖）。
    Kinoshita {
        /// 一文の最大文字数
        #[arg(long, default_value_t = 100)]
        max_sentence: usize,
        /// 一文の最大読点数
        #[arg(long, default_value_t = 4)]
        max_ten: usize,
        /// 候補報告に留め exit 0
        #[arg(long)]
        advisory: bool,
        /// 対象 file（無指定=stdin）
        files: Vec<String>,
    },
    /// 造語（Sudachi 辞書外の複合語）。要 --features coinage。
    Coinage {
        /// Sudachi 辞書 dir
        #[arg(long)]
        dict_dir: Option<PathBuf>,
        /// allow-list（除外語彙・複数可）
        #[arg(long)]
        allow: Vec<String>,
        /// stdin の unified diff（-U0）の追加行のみ判定
        #[arg(long)]
        diff: bool,
        /// strict tier（POS 所有の複合のみ）
        #[arg(long)]
        strict: bool,
        /// advisory（候補報告に留め exit 0）
        #[arg(long)]
        advisory: bool,
        /// 対象 file（無指定=stdin）
        files: Vec<String>,
    },
}

fn main() {
    match Cli::parse().command {
        Command::Codemix {
            threshold,
            allow,
            files,
        } => {
            // allow-list は明示注入（未指定＝空。識別子 / ALLCAPS の built-in 除外のみ効く）。
            // 複数 registry は union — coinage の --allow と同じ多重指定（flag 名統一・2026-07-09 F4）。
            let exempt: std::collections::HashSet<String> = allow
                .iter()
                .flat_map(|p| correo::codemix::domain_vocab(p))
                .collect();
            exit(correo::codemix::codemix(threshold, &files, &exempt));
        }
        Command::Check {
            threshold,
            allow,
            max_sentence,
            max_ten,
            format,
            files,
        } => {
            // biome check に倣う porcelain: 引数ゼロで動く・設定は correo.toml 自動発見・
            // 優先順位は CLI flag > correo.toml > 組み込み既定。findings は一度だけ集めて
            // text / json 両形式に render する（inline 抑制 = suppress.rs も一元適用）。
            // blocking は kinoshita（HARD）のみ。codemix と coinage(strict) は advisory —
            // 非 strict の列挙は discovery であって gate でない（coinage.rs 冒頭の裁定参照）。
            let (cfg_path, cfg) = match correo::config::discover() {
                Ok(x) => x,
                Err(e) => {
                    eprintln!("correo check: {e}");
                    exit(2);
                }
            };
            let threshold = threshold.or(cfg.codemix.threshold).unwrap_or(8.0);
            let max_sentence = max_sentence.or(cfg.kinoshita.max_sentence).unwrap_or(100);
            let max_ten = max_ten.or(cfg.kinoshita.max_ten).unwrap_or(4);
            // 許容語彙 = correo.toml の allow ∪ --allow registry file 群（codemix/coinage 共用）
            let mut exempt: std::collections::HashSet<String> =
                cfg.allow.iter().map(|w| w.to_lowercase()).collect();
            for p in &allow {
                exempt.extend(correo::codemix::domain_vocab(p));
            }
            // 対象: 引数なし → cwd 以下の *.md（.gitignore 準拠・ignore crate = ripgrep の walker）
            let files: Vec<String> = if files.is_empty() {
                let mut v: Vec<String> = ignore::Walk::new(".")
                    .flatten()
                    .filter(|e| e.path().extension().is_some_and(|x| x == "md"))
                    .map(|e| e.path().display().to_string())
                    .collect();
                v.sort();
                v
            } else {
                files
            };
            if files.is_empty() {
                eprintln!("correo check: 対象の *.md がありません");
                exit(2);
            }

            let mut findings: Vec<correo::report::Finding> = Vec::new();
            let mut sup: std::collections::HashMap<String, correo::suppress::Suppressions> =
                std::collections::HashMap::new();
            for f in &files {
                let text = match std::fs::read_to_string(f) {
                    Err(e) => {
                        eprintln!("correo check: {f} 読込失敗: {e} (skip)");
                        continue;
                    }
                    Ok(t) => t,
                };
                let s = correo::suppress::scan(&text);
                for p in correo::codemix::scan_paragraphs(&text, &exempt) {
                    if p.density >= threshold && !s.hit(p.line, "codemix", "latin-density") {
                        findings.push(correo::report::Finding {
                            detector: "codemix",
                            rule: "latin-density".into(),
                            file: f.clone(),
                            line: p.line,
                            severity: "advisory",
                            message: format!(
                                "{:.0} latin/100字 (JA {}) — 3-way 分類へ: {}",
                                p.density,
                                p.ja_chars,
                                p.vocab.join(" ")
                            ),
                            data: Some(serde_json::json!({
                                "density": p.density,
                                "ja_chars": p.ja_chars,
                                "vocab": p.vocab,
                            })),
                        });
                    }
                }
                for x in correo::kinoshita::scan(&text, max_sentence, max_ten) {
                    if s.hit(x.line, "kinoshita", x.rule) {
                        continue;
                    }
                    findings.push(correo::report::Finding {
                        detector: "kinoshita",
                        rule: x.rule.into(),
                        file: f.clone(),
                        line: x.line,
                        severity: match x.severity {
                            correo::kinoshita::Severity::Hard => "error",
                            correo::kinoshita::Severity::Advisory => "advisory",
                        },
                        message: x.msg,
                        data: None,
                    });
                }
                sup.insert(f.clone(), s);
            }
            #[cfg(feature = "coinage")]
            {
                let a = correo::coinage::CoinageArgs {
                    dict_dir: None,
                    allow: allow.iter().map(|p| p.display().to_string()).collect(),
                    allow_words: cfg.allow.clone(),
                    diff: false,
                    strict: true,
                    advisory: true,
                    files: files.clone(),
                };
                match correo::coinage::collect(&a) {
                    Ok((hits, _)) => {
                        for h in hits {
                            if sup
                                .get(&h.file)
                                .is_some_and(|s| s.hit(h.line, "coinage", "dictionary-coinage"))
                            {
                                continue;
                            }
                            findings.push(correo::report::Finding {
                                detector: "coinage",
                                rule: "dictionary-coinage".into(),
                                file: h.file,
                                line: h.line,
                                severity: "advisory",
                                message: format!(
                                    "「{}」は辞書見出し語でない複合 — 標準語へ書き直すか correo.toml の allow に登録",
                                    h.compound
                                ),
                                data: Some(serde_json::json!({
                                    "compound": h.compound,
                                    "components": h.components,
                                })),
                            });
                        }
                    }
                    Err(e) => eprintln!(
                        "correo check: coinage skip ({e}) — mise run setup:sudachidict で辞書を用意"
                    ),
                }
            }
            #[cfg(not(feature = "coinage"))]
            eprintln!("correo check: coinage skip（--features coinage なしの build）");

            findings.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
            let report = correo::report::Report::new(findings);
            match format {
                Format::Json => println!(
                    "{}",
                    serde_json::to_string_pretty(&report).expect("report serialize")
                ),
                Format::Text => {
                    for x in &report.findings {
                        let tag = if x.severity == "advisory" {
                            "·advisory"
                        } else {
                            ""
                        };
                        println!(
                            "{}:{}: [{}/{}{tag}] {}",
                            x.file, x.line, x.detector, x.rule, x.message
                        );
                    }
                    let cfg_note = cfg_path
                        .map(|p| format!("・設定 {}", p.display()))
                        .unwrap_or_default();
                    if report.findings.is_empty() {
                        println!("correo check: OK ({} files{cfg_note})", files.len());
                    } else {
                        println!(
                            "correo check: error {} / advisory {} ({} files{cfg_note})",
                            report.summary.error,
                            report.summary.advisory,
                            files.len()
                        );
                    }
                }
            }
            exit(if report.summary.error > 0 { 1 } else { 0 });
        }
        Command::Kinoshita {
            max_sentence,
            max_ten,
            advisory,
            files,
        } => {
            exit(correo::kinoshita::run_kinoshita(
                correo::kinoshita::KinoshitaArgs {
                    max_sentence,
                    max_ten,
                    advisory,
                    files,
                },
            ));
        }
        Command::Coinage {
            dict_dir,
            allow,
            diff,
            strict,
            advisory,
            files,
        } => {
            let a = correo::coinage::CoinageArgs {
                dict_dir,
                allow,
                allow_words: vec![],
                diff,
                strict,
                advisory,
                files,
            };
            match correo::coinage::run_coinage(a) {
                Ok(code) => exit(code),
                Err(e) => {
                    eprintln!("correo coinage: {e}");
                    exit(2);
                }
            }
        }
    }
}
