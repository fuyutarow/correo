// codemix.rs — code-mixing（ルー語）密度 flagger（MIX tier・corpus-locate 層）。
// scan_paragraphs は records-prose gate（gates.rs）と共有する測定核。
use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// 密度から除外する既知 domain 語彙（＝「登録制」の exempt 源）を allow-md ファイルから作る。
/// coinage の allow 源と同一形式の統制語彙 registry を codemix も consult する（融通機構・2026-07-06）。
/// 抽出＝登録行 `| \`term …\` | gloss |` の term セルと gloss の latin token（lower・len≥2）。
/// 呼び出し側が allow-md パスを渡して注入する（汎用 binary は --allow・複数可＝union。
/// ファイル不在なら空 exempt へ劣化）。
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
/// 散文抽出（fence / 構造行 / inline strip / 段落 split / bullet 行群）は `prose.rs` の
/// 単一 home に委譲（2026-07-09 抽出）— ここに残るのは codemix 固有の**測定**だけ。
/// JA 40 字未満の単位は密度が無意味なので除外（README 判定床）。
pub fn scan_paragraphs(text: &str, exempt: &HashSet<String>) -> Vec<Para> {
    // token を full で捕え（識別子を割らない）、散文の code-mix だけを数える。除外:
    //   (1) 識別子＝`_` か数字を含む token（collective_only_residual・P3b・c_adaptive_pair）。
    //   (2) ALLCAPS 略語＝HTTP/API/YKL/PPT/RDM/CRB/JW 等の domain 略語（構造で判る・列挙不要＝融通）。
    //   (3) allow-list 登録語＝exempt（登録制・HTTP/JSON 等は allow-list 登録で「もちろんそのまま」）。
    //   (4) 数式・列挙の破片＝1 字 token（O(n²) の o と n・変数 x）と小文字ローマ数字（(i)(ii)(iii)）。
    // dogfooding 2026-07-06: (1) で ledger 誤検出、(2)(3) で domain 語誤検出（precision ~5%）を潰す。
    // dogfooding 2026-07-09: (4) は Bitter-Lesson.md 実走で発見 — 数式の破片が密度を水増しした。
    let latin = Regex::new(r"[a-zA-Z][a-zA-Z0-9_-]*").unwrap();
    let is_identifier = |t: &str| t.contains('_') || t.chars().any(|c| c.is_ascii_digit());
    let is_acronym = |t: &str| t.len() >= 2 && t.chars().all(|c| c.is_ascii_uppercase());
    let is_math_fragment =
        |t: &str| t.chars().count() < 2 || t.chars().all(|c| matches!(c, 'i' | 'v' | 'x'));
    let ja = Regex::new(r"[\p{Hiragana}\p{Katakana}\p{Han}]").unwrap();
    let mut out = Vec::new();
    for u in crate::prose::prose_units(text) {
        let ja_n = ja.find_iter(&u.text).count();
        if ja_n < 40 {
            continue;
        }
        let toks: Vec<String> = latin
            .find_iter(&u.text)
            .map(|m| m.as_str())
            .filter(|t| !is_identifier(t) && !is_acronym(t)) // 識別子・ALLCAPS 略語は散文でない
            .map(|t| t.to_lowercase())
            .filter(|t| !is_math_fragment(t)) // 数式・列挙の破片は散文でない
            .filter(|t| !exempt.contains(t)) // allow-list 登録の domain 語は exempt（登録制）
            .collect();
        let density = toks.len() as f64 * 100.0 / ja_n as f64;
        let mut vocab = toks;
        vocab.sort();
        vocab.dedup();
        out.push(Para {
            line: u.line,
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
                    "⚑ {label}L{}: {:.0} latin per 100 JA chars (JA {} chars)",
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
                Err(e) => eprintln!("correo codemix: {f} 読込失敗: {e} (skip)"),
                Ok(t) => scan(&format!("{f} "), &t),
            }
        }
    }
    if hot == 0 {
        println!(
            "CODEMIX PASS: no paragraph above {threshold:.0} latin per 100 JA chars (advisory)"
        );
    } else {
        println!(
            "CODEMIX: {hot} hot paragraphs (advisory — judge triages each token: domain / mention / gratuitous→rewrite)"
        );
    }
    0 // advisory：常に exit 0
}

// ───────────────── characterization tests（抽出前の behavior 固定・2026-07-07）─────────────────
// correo 抽出のリスク精査（oracle-parity レンズ）が「scan_paragraphs は cargo test 被覆ゼロ・
// filter 順序を反転すると ALLCAPS が計上され密度が静かに膨張」と指摘。移設で bit ずれても
// 気付けるよう、各分岐（識別子/ALLCAPS/exempt 除外・40字skip・行番号・表/見出し strip・inline code
// strip）を1ケースずつ固定する。この test は codemix.rs と共に correo へ移送し緑を保つ。
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
    fn strips_fence_containing_blank_lines_and_keeps_line_numbers() {
        // 回帰 (2026-07-09 F2): 空行を含む fence は段落分割で泣き別れ、中身が地の文として
        // 計上されていた。修正後は fence を分割前に全文から落とす（同数改行置換＝行番号不変）。
        let exempt = HashSet::new();
        // fence 中身に JA≥40 の行（現実には JA コメント付き code block）が無いと 40字床が
        // 偶然救ってバグが見えない — JA を含む fence が真の再現形。
        let text = format!(
            "{} realword\n\n```\n{} fenceword framework\n\ncodeword\n```\n\n{} tailword",
            ja(45),
            ja(45),
            ja(45)
        );
        let paras = scan_paragraphs(&text, &exempt);
        assert_eq!(paras.len(), 2, "fence が段落として計上された");
        for p in &paras {
            assert!(
                !p.vocab.contains(&"codeword".to_string())
                    && !p.vocab.contains(&"fenceword".to_string())
                    && !p.vocab.contains(&"framework".to_string()),
                "fence 中身が地の文に漏れた: {:?}",
                p.vocab
            );
        }
        assert_eq!(paras[0].line, 1);
        assert_eq!(paras[1].line, 9, "fence 除去で行番号がずれた"); // 1..7=文+fence, 9=tail
    }

    #[test]
    fn paragraph_line_skips_leading_blank_lines() {
        // 回帰 (2026-07-09 F5a): 3 連続改行で次段落の開始行が空行を指していた off-by-one。
        let exempt = HashSet::new();
        let text = format!("{} framework\n\n\n{} gratuitous", ja(45), ja(45));
        let paras = scan_paragraphs(&text, &exempt);
        assert_eq!(paras.len(), 2);
        assert_eq!(paras[1].line, 4, "空行でなく本文行を指すべき"); // 1=文,2-3=空行,4=本文
    }

    #[test]
    fn whitespace_only_lines_are_paragraph_boundaries() {
        // 回帰 (2026-07-09 F5b): 旧実装は "\n \n"（space 1個）のみ対応。tab/複数 space/全角も境界。
        let exempt = HashSet::new();
        let text = format!("{} framework\n\t \n{} gratuitous", ja(45), ja(45));
        let paras = scan_paragraphs(&text, &exempt);
        assert_eq!(paras.len(), 2, "空白のみ行が段落境界にならず");
        assert_eq!(paras[1].line, 3);
    }

    #[test]
    fn groups_consecutive_bullet_paragraphs_over_the_40_char_floor() {
        // F1(b) 2026-07-09: LLM slop こそ短い bullet に湧くのに、blank 行区切りの箇条書きは
        // 各段落が 40字床を割って丸ごと不可視だった。連続する bullet 段落は 1 行群として集計する。
        let exempt = HashSet::new();
        let text = format!(
            "- {} framework\n\n- {} gratuitous\n\n- {} slopword",
            ja(20),
            ja(15),
            ja(18)
        );
        let paras = scan_paragraphs(&text, &exempt);
        assert_eq!(paras.len(), 1, "bullet 行群が 1 単位に集計されていない");
        assert_eq!(paras[0].line, 1);
        assert_eq!(paras[0].ja_chars, 53);
        for w in ["framework", "gratuitous", "slopword"] {
            assert!(paras[0].vocab.contains(&w.to_string()), "{w} が欠落");
        }
    }

    #[test]
    fn bullet_group_does_not_merge_with_prose_paragraph() {
        // 行群化は bullet 同士のみ — 直前直後の地の文段落とは併合しない。
        let exempt = HashSet::new();
        let text = format!(
            "{} proseword\n\n- {} bulletword\n\n- {} another",
            ja(45),
            ja(25),
            ja(25)
        );
        let paras = scan_paragraphs(&text, &exempt);
        assert_eq!(paras.len(), 2, "prose と bullet が併合された/分離しすぎた");
        assert!(paras[0].vocab.contains(&"proseword".to_string()));
        assert!(paras[1].vocab.contains(&"bulletword".to_string()));
        assert_eq!(paras[1].line, 3);
    }

    #[test]
    fn math_fragments_are_not_prose_tokens() {
        // O(n²) の o/n・変数 x・列挙 (i)(ii)(iii) は数式・記号の破片 — 散文の code-mix でない
        // （Bitter-Lesson.md 実走で発見した密度水増し・2026-07-09）。実語 word は残ること。
        let exempt = HashSet::new();
        let text = format!("{} O(n) の x と (i) (ii) (iii) を word で論じる。", ja(45));
        let paras = scan_paragraphs(&text, &exempt);
        assert_eq!(
            paras[0].vocab,
            vec!["word".to_string()],
            "破片が語彙に混入: {:?}",
            paras[0].vocab
        );
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
