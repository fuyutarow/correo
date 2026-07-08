// codemix.rs — code-mixing（ルー語）密度 flagger（MIX tier・corpus-locate 層）。
// scan_paragraphs は records-prose gate（gates.rs）と共有する測定核。
use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// 密度から除外する既知 domain 語彙（＝「登録制」の exempt 源）を allow-md ファイルから作る。
/// coinage の allow 源と同一形式の統制語彙 registry を codemix も consult する（融通機構・2026-07-06）。
/// 抽出＝登録行 `| \`term …\` | gloss |` の term セルと gloss の latin token（lower・len≥2）。
/// 呼び出し側が注入: 呼び出し側が allow-md パスを渡す（呼び出し側が allow-md を渡す（例:
/// 汎用 binary は --exempt。ファイル不在なら空 exempt へ劣化）。
pub fn domain_vocab(allow_md: &Path) -> HashSet<String> {
    let tax = fs::read_to_string(allow_md).unwrap_or_default();
    let row = Regex::new(r"^\|\s*`").unwrap();
    let latin = Regex::new(r"[a-zA-Z][a-zA-Z-]*").unwrap();
    let mut v = HashSet::new();
    for line in tax.lines().filter(|l| row.is_match(l)) {
        for m in latin.find_iter(line) {
            let t = m.as_str().to_lowercase();
            if t.len() >= 2 {
                v.insert(t);
            }
        }
    }
    v
}

/// 地の文段落の測定結果。
pub struct Para {
    /// 段落の開始行（1 始まり）。序数でなく行番号＝編集で直接開ける（dogfooding 2026-07-06:
    /// 序数 ¶74 は 237 行の doc で探せず実害だった瑕疵の是正）。
    pub line: usize,
    pub density: f64,
    pub ja_chars: usize,
    pub vocab: Vec<String>,
}

/// text を段落へ割り、地の文だけの latin 密度（latin/100 JA字）を全段落分返す。
/// 判定対象外として strip: code fence / inline code / URL / §ref / ledger id、および
/// **構造行**（markdown 表 `|`・見出し `#`）— 表は英語 term 列で密度が地の文の意味を失う
/// （dogfooding 実測: 表 347・実測 830 = 表行 artefact・2026-07-06）。
/// JA 40 字未満の段落は密度が無意味なので除外。
pub fn scan_paragraphs(text: &str, exempt: &HashSet<String>) -> Vec<Para> {
    let strip = Regex::new(r"(?s)```.*?```|`[^`]*`|https?://\S+|§\S+|R\d{4}_\d+|IF-\d").unwrap();
    // token を full で捕え（識別子を割らない）、散文の code-mix だけを数える。除外:
    //   (1) 識別子＝`_` か数字を含む token（collective_only_residual・P3b・c_adaptive_pair）。
    //   (2) ALLCAPS 略語＝HTTP/API/YKL/PPT/RDM/CRB/JW 等の domain 略語（構造で判る・列挙不要＝融通）。
    //   (3) allow-list 登録語＝exempt（登録制・HTTP/JSON 等は allow-list 登録で「もちろんそのまま」）。
    // dogfooding 2026-07-06: (1) で ledger 誤検出、(2)(3) で domain 語誤検出（precision ~5%）を潰す。
    let latin = Regex::new(r"[a-zA-Z][a-zA-Z0-9_-]*").unwrap();
    let is_identifier = |t: &str| t.contains('_') || t.chars().any(|c| c.is_ascii_digit());
    let is_acronym = |t: &str| t.len() >= 2 && t.chars().all(|c| c.is_ascii_uppercase());
    let ja = Regex::new(r"[\p{Hiragana}\p{Katakana}\p{Han}]").unwrap();
    // 段落境界（空行）を正規化し、開始行を追跡する。`\n \n`（空白のみの行）も境界扱い ──
    // 置換で改行数は不変（どちらも 2 改行）ゆえ行カウントは正確。
    let normalized = text.replace("\n \n", "\n\n");
    let mut out = Vec::new();
    let mut line = 1usize;
    for para in normalized.split("\n\n") {
        let start_line = line;
        line += para.matches('\n').count() + 2; // 段落内の改行 ＋ 区切りの "\n\n"
        // 構造行を落として地の文だけ残す（段落全体が表/見出しなら空になり skip される）
        let prose: String = para
            .lines()
            .filter(|l| {
                let t = l.trim_start();
                !t.starts_with('|') && !t.starts_with('#')
            })
            .collect::<Vec<_>>()
            .join("\n");
        let clean = strip.replace_all(&prose, "");
        let ja_n = ja.find_iter(&clean).count();
        if ja_n < 40 {
            continue;
        }
        let toks: Vec<String> = latin
            .find_iter(&clean)
            .map(|m| m.as_str())
            .filter(|t| !is_identifier(t) && !is_acronym(t)) // 識別子・ALLCAPS 略語は散文でない
            .map(|t| t.to_lowercase())
            .filter(|t| !exempt.contains(t)) // allow-list 登録の domain 語は exempt（登録制）
            .collect();
        let density = toks.len() as f64 * 100.0 / ja_n as f64;
        let mut vocab = toks;
        vocab.sort();
        vocab.dedup();
        out.push(Para {
            line: start_line,
            density,
            ja_chars: ja_n,
            vocab,
        });
    }
    out
}

// ───────────────── codemix（ルー語 密度 flagger・MIX tier・corpus 構造 proxy）─────────────────
/// 段落ごとの latin-token 密度が閾値超で flag し latin 語彙を列挙する。
/// list-free の corpus-locate 層（頻度/perplexity は domain-vs-gratuitous を切れないと実証済み・
/// linting-prose machine-floor）。分類（domain 用語／pinned token／gratuitous calque）は LLM-judge が担う。
/// 旧 linting-prose scripts/codemix-flag.py の Rust 移植。
pub fn codemix(threshold: f64, files: &[String], exempt: &HashSet<String>) -> i32 {
    let mut hot = 0usize;
    let mut scan = |label: &str, text: &str| {
        for p in scan_paragraphs(text, exempt) {
            if p.density >= threshold {
                hot += 1;
                println!(
                    "⚑ {label}L{}: {:.0} latin/100字 (JA {} chars)",
                    p.line, p.density, p.ja_chars
                );
                println!(
                    "   classify 3-way (domain/pinned/gratuitous): {}",
                    p.vocab.join(" ")
                );
            }
        }
    };

    if files.is_empty() {
        use std::io::Read;
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf).ok();
        scan("", &buf);
    } else {
        for f in files {
            match fs::read_to_string(f) {
                Err(e) => eprintln!("ja-slop-lint codemix: {f} 読込失敗: {e} (skip)"),
                Ok(t) => scan(&format!("{f} "), &t),
            }
        }
    }
    if hot == 0 {
        println!("CODEMIX PASS: latin 密度 {threshold:.0}/100字 超の段落なし（advisory）");
    } else {
        println!(
            "CODEMIX: {hot} hot 段落（advisory=MIX・LLM-judge が 3-way 分類→gratuitous のみ修正/prh 焼き）"
        );
    }
    0 // advisory：常に exit 0
}

// ───────────────── characterization tests（抽出前の behavior 固定・2026-07-07）─────────────────
// ja-slop-lint 抽出のリスク精査（oracle-parity レンズ）が「scan_paragraphs は cargo test 被覆ゼロ・
// filter 順序を反転すると ALLCAPS が計上され密度が静かに膨張」と指摘。移設で bit ずれても
// 気付けるよう、各分岐（識別子/ALLCAPS/exempt 除外・40字skip・行番号・表/見出し strip・inline code
// strip）を1ケースずつ固定する。この test は codemix.rs と共に ja-slop-lint へ移送し緑を保つ。
#[cfg(test)]
mod tests {
    use super::*;

    fn ja(n: usize) -> String {
        "あ".repeat(n)
    }

    #[test]
    fn excludes_identifiers_acronyms_exempt_keeps_gratuitous() {
        let mut exempt = HashSet::new();
        exempt.insert("postgres".to_string());
        let text = format!(
            "{} framework collective_only_residual HTTP postgres gratuitous",
            ja(45)
        );
        let paras = scan_paragraphs(&text, &exempt);
        assert_eq!(paras.len(), 1);
        let v = &paras[0].vocab;
        assert!(v.contains(&"framework".to_string()));
        assert!(v.contains(&"gratuitous".to_string()));
        assert!(
            !v.iter().any(|t| t.contains('_')),
            "snake_case 識別子が漏れた: {v:?}"
        );
        assert!(!v.contains(&"http".to_string()), "ALLCAPS が漏れた");
        assert!(!v.contains(&"postgres".to_string()), "exempt が漏れた");
        // 密度は生 token 数（dedup 前）/JA*100。counted={framework,gratuitous}=2, JA=45。
        assert!((paras[0].density - 2.0 * 100.0 / 45.0).abs() < 1e-9);
        assert_eq!(paras[0].ja_chars, 45);
    }

    #[test]
    fn skips_paragraphs_under_40_ja_chars() {
        let exempt = HashSet::new();
        let text = "短い framework gratuitous slop"; // JA=2 < 40
        assert!(scan_paragraphs(text, &exempt).is_empty());
    }

    #[test]
    fn reports_paragraph_start_line_not_ordinal() {
        let exempt = HashSet::new();
        let text = format!("{} framework\n\n{} gratuitous", ja(45), ja(45));
        let paras = scan_paragraphs(&text, &exempt);
        assert_eq!(paras.len(), 2);
        assert_eq!(paras[0].line, 1);
        assert_eq!(paras[1].line, 3); // 空行を挟んで 3 行目
    }

    #[test]
    fn strips_table_and_heading_lines() {
        let exempt = HashSet::new();
        let text = format!(
            "# headingword\n| tableword framework |\n{} realprose",
            ja(45)
        );
        let paras = scan_paragraphs(&text, &exempt);
        assert_eq!(paras.len(), 1);
        let v = &paras[0].vocab;
        assert!(v.contains(&"realprose".to_string()));
        assert!(
            !v.contains(&"headingword".to_string()),
            "見出しが strip されず"
        );
        assert!(!v.contains(&"framework".to_string()), "表行が strip されず");
    }

    #[test]
    fn strips_inline_code_and_refs() {
        let exempt = HashSet::new();
        let text = format!("{} `codeword framework` §5 DOC_1234 realword", ja(45));
        let paras = scan_paragraphs(&text, &exempt);
        let v = &paras[0].vocab;
        assert!(v.contains(&"realword".to_string()));
        assert!(
            !v.contains(&"codeword".to_string()),
            "inline code が strip されず"
        );
        assert!(
            !v.contains(&"framework".to_string()),
            "inline code 内が残った"
        );
    }
}
