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
    /// GitHub Actions の workflow command（PR の該当行に inline 注釈が付く）
    Github,
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
    /// ルー語密度（地の文の latin / 100字）。file 無し=stdin。通常は `check` を使う（correo.toml の allow/抑制が効く）。本 subcommand は設定を読まず明示引数のみ。
    Codemix {
        /// 密度閾値（latin / 100 JA字）
        #[arg(long, default_value_t = 8.0)]
        threshold: f64,
        /// allow-list（除外語彙・統制語彙 registry の .md・複数可。coinage と同名 flag に統一）
        #[arg(long, alias = "exempt")]
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
        #[arg(long, alias = "exempt")]
        allow: Vec<PathBuf>,
        /// readability: 一文の最大文字数（既定 100・correo.toml で設定可）
        #[arg(long)]
        max_sentence: Option<usize>,
        /// readability: 一文の最大読点数（既定 4・correo.toml で設定可）
        #[arg(long)]
        max_ten: Option<usize>,
        /// 出力形式（text=人向け / json=judge 連携・機械可読）
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
        /// 機械的に安全な修正を適用して書き戻す（現在: `することができ`→`でき`）
        #[arg(long)]
        write: bool,
        /// 対象 file（省略時: cwd 以下の *.md を .gitignore 準拠で全走査）
        files: Vec<String>,
    },
    /// correo.toml の雛形を生成する（既にあれば何もしない）。
    Init,
    /// coinage 用の Sudachi 辞書を ~/.cache/correo へ取得する（brew は辞書非同梱・CLI が管理）。
    Setup {
        /// 既存でも取り直す
        #[arg(long)]
        force: bool,
    },
    /// 可読性・トーンの床（規準は木下是雄: 文長・読点過多・文体混在・二重否定・ぼかし・指示語）。
    Readability {
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
    /// 動詞カルク（英語動詞＋する/される の接合: deployする）。file 無し=stdin。hook 用の単体経路。
    Calque {
        /// 候補報告に留め exit 0（既定は hit で exit 1 — Stop hook の block 用）
        #[arg(long)]
        advisory: bool,
        /// 対象 file（無指定=stdin）
        files: Vec<String>,
    },
    /// 造語（Sudachi 辞書外の複合語）。要 --features coinage。通常は `check`（correo.toml の allow/corpus 照合が効く）。--strict 無しは棚卸し用の discovery tier（機能複合も全列挙）。
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

const INIT_TEMPLATE: &str = r#"#:schema https://raw.githubusercontent.com/fuyutarow/correo/main/schemas/correo.schema.json
# correo の設定。`correo check` が cwd から根へ辿って自動発見する。無くても既定値で動く。

# judge（人か LLM）の裁定を経た語だけを登録する — 造語の逃げ場にしない。
# 一回きりの言及は <!-- correo-ignore --> で行単位に抑制する。
allow = [
  # "ルー語",   # 例: 俗称として定着（言い換えると通じない）
]

# 確定した造語の禁止（HARD・exit 1）。値は書き直しの案。
[deny]
# "機械床" = "「機械的に判定できる」等へ書き直す"

[codemix]
threshold = 8.0

[readability]
max-sentence = 100
max-ten = 4
"#;

fn main() {
    match Cli::parse().command {
        Command::Calque { advisory, files } => {
            exit(correo::calque::run_calque(&files, advisory));
        }
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
        Command::Setup { force } => exit(correo::setup::run(force)),
        Command::Init => {
            let p = std::path::Path::new("correo.toml");
            if p.exists() {
                eprintln!("correo init: correo.toml は既にある — 何もしない");
                exit(2);
            }
            std::fs::write(p, INIT_TEMPLATE).expect("correo.toml の書き込みに失敗");
            println!(
                "correo init: correo.toml を作成した（#:schema でエディタ補完が効く）。correo check で検査を開始"
            );
            exit(0);
        }
        Command::Check {
            threshold,
            allow,
            max_sentence,
            max_ten,
            format,
            write,
            files,
        } => {
            // biome check に倣う porcelain: 引数ゼロで動く・設定は correo.toml 自動発見・
            // 優先順位は CLI flag > correo.toml > 組み込み既定。findings は一度だけ集めて
            // text / json 両形式に render する（inline 抑制 = suppress.rs も一元適用）。
            // blocking は readability（HARD）のみ。codemix と coinage(strict) は advisory —
            // 非 strict の列挙は discovery であって gate でない（coinage.rs 冒頭の裁定参照）。
            let (cfg_path, cfg) = match correo::config::discover() {
                Ok(x) => x,
                Err(e) => {
                    eprintln!("correo check: {e}");
                    exit(2);
                }
            };
            let threshold = threshold.or(cfg.codemix.threshold).unwrap_or(8.0);
            let max_sentence = max_sentence.or(cfg.readability.max_sentence).unwrap_or(100);
            let max_ten = max_ten.or(cfg.readability.max_ten).unwrap_or(4);
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
                eprintln!("correo check: no *.md targets found");
                exit(2);
            }

            // deny = 組み込み slop 常套句（出荷時の意見のある既定）∪ ユーザの確定裁定。
            // allow に語を書けばどちらも個別解除できる（組み込みへの拒否権はユーザが持つ）。
            let allow_set: std::collections::HashSet<&str> =
                cfg.allow.iter().map(String::as_str).collect();
            let mut builtin_deny = correo::deny::builtin();
            builtin_deny.retain(|w, _| !allow_set.contains(w.as_str()));
            let mut user_deny = cfg.deny.clone();
            user_deny.retain(|w, _| !allow_set.contains(w.as_str()));

            // メタファー語彙表（data・optional）: correo.toml 指定（相対は toml の場所基準）
            // > exe 相対 share/correo/（brew 配布）。無ければ metaphor-density は沈黙。
            let metaphor_lex: Vec<(String, String)> = cfg
                .rhetoric
                .metaphor_lexicon
                .as_ref()
                .map(|p| {
                    if p.is_relative() {
                        cfg_path
                            .as_ref()
                            .and_then(|c| c.parent())
                            .map(|d| d.join(p))
                            .unwrap_or_else(|| p.clone())
                    } else {
                        p.clone()
                    }
                })
                .or_else(|| {
                    std::env::current_exe().ok().and_then(|exe| {
                        exe.parent()
                            .map(|d| d.join("../share/correo/metaphor-lex.tsv"))
                    })
                })
                .map(|p| correo::rhetoric::load_lexicon(&p))
                .unwrap_or_default();

            let mut findings: Vec<correo::report::Finding> = Vec::new();
            let mut fixed = 0usize;
            let mut sup: std::collections::HashMap<String, correo::suppress::Suppressions> =
                std::collections::HashMap::new();
            for f in &files {
                let mut text = match std::fs::read_to_string(f) {
                    Err(e) => {
                        eprintln!("correo check: {f} read failed: {e} (skip)");
                        continue;
                    }
                    Ok(t) => t,
                };
                if write {
                    // 修正 → 書き戻し → 修正後 text を検査（直した違反は報告に残らない）
                    let (out, n) = correo::readability::fix(&text);
                    if n > 0 {
                        if let Err(e) = std::fs::write(f, &out) {
                            eprintln!("correo check: {f} write failed: {e}");
                        } else {
                            fixed += n;
                            text = out;
                        }
                    }
                }
                let s = correo::suppress::scan(&text);
                let ctx = correo::pipeline::Ctx {
                    max_sentence,
                    max_ten,
                    threshold,
                    exempt: &exempt,
                    allow_set: &allow_set,
                    metaphor_lex: &metaphor_lex,
                    user_deny: &user_deny,
                    builtin_deny: &builtin_deny,
                };
                findings.extend(correo::pipeline::scan_document(f, &text, &s, &ctx));
                sup.insert(f.clone(), s);
            }
            #[cfg(feature = "coinage")]
            {
                // corpus 照合（第二の証拠）: 辞書に無い複合でも実コーパスに在れば自然（物理層）、
                // 無ければ造語として error へ昇格（機械床）— deny の手書き複写を不要にする。
                // corpus 設定があるのに file が無いのは環境の未整備 — 大声で警告して advisory へ
                // 劣化する（zero-config の頑健さを壊さない）。
                let corpus: Option<std::collections::HashSet<String>> =
                    cfg.coinage.corpus.as_ref().and_then(|p| {
                        let p = match p.strip_prefix("~/") {
                            Ok(rest) => std::env::var("HOME")
                                .map(|h| std::path::PathBuf::from(h).join(rest))
                                .unwrap_or_else(|_| p.clone()),
                            Err(_) => {
                                if p.is_relative() {
                                    cfg_path
                                        .as_ref()
                                        .and_then(|c| c.parent())
                                        .map(|d| d.join(p))
                                        .unwrap_or_else(|| p.clone())
                                } else {
                                    p.clone()
                                }
                            }
                        };
                        match correo::coinage::load_corpus(&p) {
                            Ok(s) => Some(s),
                            Err(e) => {
                                eprintln!(
                                    "correo check: {e} — run `mise run setup:corpus` to fetch (coinage stays advisory until then)"
                                );
                                None
                            }
                        }
                    });
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
                            // corpus の役割は「実在の証明で候補を消す」こと【だけ】。不在を error に
                            // 昇格させない — 使用例/実用文 級の生産的複合はどんな有限語彙表にも
                            // 載らず（full lex 387万語で実測 0・2026-07-09）、不在≠造語。造語の
                            // 確定は judge の領分で、その裁定の永続化が [deny]。
                            if let Some(c) = &corpus
                                && c.contains(&h.compound)
                            {
                                continue; // 実在の証明（物理層・混種語 等）＝自然
                            }
                            if cfg.deny.contains_key(&h.compound) {
                                continue; // deny が error として報告済み — 二重報告しない
                            }
                            let (severity, message) = (
                                "advisory",
                                format!(
                                    "「{}」 is not a dictionary headword — judge: natural→ignore / keep→allow / confirmed coinage→deny or rewrite",
                                    h.compound
                                ),
                            );
                            findings.push(correo::report::Finding {
                                detector: "coinage",
                                rule: "dictionary-coinage".into(),
                                file: h.file,
                                line: h.line,
                                severity,
                                message,
                                data: Some(serde_json::json!({
                                    "compound": h.compound,
                                    "components": h.components,
                                })),
                            });
                        }
                    }
                    Err(e) => eprintln!(
                        "correo check: coinage skipped ({e}) — run `mise run setup:sudachidict` to install the dictionary"
                    ),
                }
            }
            #[cfg(not(feature = "coinage"))]
            eprintln!("correo check: coinage skipped (built without --features coinage)");

            findings.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
            let report = correo::report::Report::new(findings);
            match format {
                Format::Json => println!(
                    "{}",
                    serde_json::to_string_pretty(&report).expect("report serialize")
                ),
                Format::Github => {
                    // GitHub Actions workflow command — PR の該当行へ inline 注釈（vale の
                    // reviewdog 連携に相当する CI DevX を追加依存なしで）。message の % は
                    // command 構文のため escape。
                    for x in &report.findings {
                        let level = if x.severity == "error" {
                            "error"
                        } else {
                            "notice"
                        };
                        let msg = x.message.replace('%', "%25");
                        println!(
                            "::{level} file={},line={},title=correo {}/{}::{msg}",
                            x.file, x.line, x.detector, x.rule
                        );
                    }
                }
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
                        .map(|p| format!(", config {}", p.display()))
                        .unwrap_or_default();
                    let fix_note = if fixed > 0 {
                        format!("・fixed {fixed}")
                    } else {
                        String::new()
                    };
                    if report.findings.is_empty() {
                        println!(
                            "correo check: OK ({} files{fix_note}{cfg_note})",
                            files.len()
                        );
                    } else {
                        println!(
                            "correo check: error {} / advisory {} ({} files{fix_note}{cfg_note})",
                            report.summary.error,
                            report.summary.advisory,
                            files.len()
                        );
                    }
                }
            }
            exit(if report.summary.error > 0 { 1 } else { 0 });
        }
        Command::Readability {
            max_sentence,
            max_ten,
            advisory,
            files,
        } => {
            exit(correo::readability::run_readability(
                correo::readability::ReadabilityArgs {
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
