//! correo CLI — 薄い entry。日本語散文の質を stdin/file で検出する。
//! 検出ロジックは lib（codemix / coinage）が持ち、この binary は薄く委譲するだけ。calque は順次追加。
//! 引数解釈は clap derive（旧・手書き iter parse を置換 — `--flag=value`・`--`・未知フラグ error・
//! `--help` を得る）。hook はこの binary へ薄く委譲する。
use std::path::PathBuf;
use std::process::exit;

use clap::{Parser, Subcommand};

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
    /// 全検出器を一括実行（codemix=advisory・kinoshita=HARD・coinage=辞書があれば。biome check に倣う集約 gate）。
    Check {
        /// codemix の密度閾値（latin / 100 JA字）
        #[arg(long, default_value_t = 8.0)]
        threshold: f64,
        /// allow-list（codemix / coinage 共用・複数可）
        #[arg(long)]
        allow: Vec<PathBuf>,
        /// kinoshita: 一文の最大文字数
        #[arg(long, default_value_t = 100)]
        max_sentence: usize,
        /// kinoshita: 一文の最大読点数
        #[arg(long, default_value_t = 4)]
        max_ten: usize,
        /// 対象 file（stdin 不可 — 複数検出器が同じ入力を読むため file 指定必須）
        #[arg(required = true)]
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
            files,
        } => {
            // 1 コマンド＝全 gate。blocking は kinoshita（HARD 床）のみ。codemix は advisory、
            // coinage は strict+advisory（locate 層 — 非 strict の列挙は discovery であって gate
            // でない: 本手法/三回/反復的 等の自然な複合まで数える。blocking は prh residue の分業・
            // coinage.rs 冒頭の裁定コメント参照）。辞書が無ければ skip を明示 — check は「使える床を
            // 全部張る」であり、環境不足で全体を殺さない（個別 subcommand は従来の exit 契約のまま）。
            let exempt: std::collections::HashSet<String> = allow
                .iter()
                .flat_map(|p| correo::codemix::domain_vocab(p))
                .collect();
            let mut code = correo::codemix::codemix(threshold, &files, &exempt); // 常に 0
            code = code.max(correo::kinoshita::run_kinoshita(
                correo::kinoshita::KinoshitaArgs {
                    max_sentence,
                    max_ten,
                    advisory: false,
                    files: files.clone(),
                },
            ));
            #[cfg(feature = "coinage")]
            {
                let a = correo::coinage::CoinageArgs {
                    dict_dir: None,
                    allow: allow.iter().map(|p| p.display().to_string()).collect(),
                    diff: false,
                    strict: true,
                    advisory: true,
                    files: files.clone(),
                };
                match correo::coinage::run_coinage(a) {
                    Ok(c) => code = code.max(c),
                    Err(e) => eprintln!(
                        "correo check: coinage skip ({e}) — mise run setup:sudachidict で辞書を用意"
                    ),
                }
            }
            #[cfg(not(feature = "coinage"))]
            eprintln!("correo check: coinage skip（--features coinage なしの build）");
            exit(code);
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
