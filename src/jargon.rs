// jargon.rs — 内輪語の輸出検査（jargon-export・2026-07-11・register 軸の第二波）。
//
// 出自: 統制語彙表（taxonomy）を持つ実務プロジェクトの実測事例（2026-07）。免除辞書
// （correo.toml の allow・coinage の corpus）が書き手プロジェクトの統制語彙表一本であるとき、
// 「書き手の語彙 = 許される語彙」が構造に焼き込まれる。internal 読者（同じ語彙を共有する
// チーム）ではこれで正しいが、external 読者（語彙を共有しない広い読者）では役割が逆転する ──
// **統制語彙表に載る語こそが「定義なしで輸出された内輪語」の最優先候補**になる。共有語彙とは
// 読者と共有された語彙であって、書き手の語彙表そのものではない。
//
// 設計: 語彙表（例: `docs/handbook/taxonomy.md` のような統制語彙表 file）の**第 1 列**に
// 載る語だけを候補とする。codemix の domain_vocab（--allow 用）は行全体から latin token を
// 拾う（allow-list の用途に合う広い抽出）が、jargon-export は「その語が定義された語」だけを
// 見たいので第 1 列限定の専用抽出を持つ（home パスや gloss 文中の英単語まで語彙とみなすと
// 過検出になる）。
//
// register が external のときだけ発火する（internal の既定動作＝免除リストとしての役割は
// pipeline.rs 側で不変。jargon-export はここでは扱わない ── 二重責務にしない）。
//
// 判定（advisory・per-instance・locate 哲学＝最終判定は judge）:
//   各語の初出について、以下のどちらかを満たせば合格:
//   (a) 直前直後の近傍（同一文相当・前後 ~40 字）に日本語の言い換えが括弧で添えられている
//       （「campaign（測定の実施計画）」「（測定の実施計画）campaign」のどちらの順も可）、
//       または「とは」による定義文が同一行内にある。
//   (b) 文書内に用語表（markdown table / HTML table・dl）があり、その第 1 列にその語が載る。
// 用語表判定は生テキスト（strip_html 前 / clean_lines 前）に対して行う ── clean_lines は
// markdown の `|` 行を空行化し、strip_html は block tag 境界を潰すため、いずれも表の列構造を
// 破壊する。よって scan は「表構造の探索」と「出現・定義徴候の探索」を別テキストで行う:
// 前者は raw（HTML markup / markdown 記法を保持したまま）、後者は clean_lines 後。
use std::collections::HashSet;

use regex::Regex;

pub struct Finding {
    pub line: usize,
    pub rule: &'static str,
    pub msg: String,
}

/// 語彙表（taxonomy.md 形式: `| \`term\` | home | gloss | ... |`）から第 1 列の**見出し語**を
/// 抽出する。domain_vocab（codemix.rs）と異なり行全体でなく第 1 列のセルだけを見て、かつセル内も
/// 先頭トークン（最初の空白/括弧の前まで）だけを候補にする ── taxonomy.md の第 1 列は
/// 「`thm:main (yield ⇔ tr(W_spec M⁻¹) L-optimal equivalence)`」のような説明的な複合表現であり、
/// セル全体から latin token を拾うと gloss 相当の文中語（yield・count・bound 等）や POVM/Fisher/
/// SDP/qubit のような広く定着した標準語（README が「量子ビット・較正・忠実度・POVM・Fisher 情報・
/// SDP などの標準語」として明示的に語彙表から除外する語）まで巻き込む過検出になる（実測 2026-07-11:
/// セル全体では 191 語・先頭トークン限定では 40 語で、POVM/Fisher/yield/SDP/qubit の埋め込み出現は
/// 落ちる）。先頭トークンは「その行が定義する概念の主見出し」に対応する設計判断 — 完全な精度では
/// ないが（`qubit 等価定理 …` のように見出し自体が説明的な行は依然拾う）、gloss 文中への漏れ出しは
/// 遮断する。
pub fn load_terms(path: &std::path::Path) -> Vec<String> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let row = Regex::new(r"^\|\s*`").unwrap();
    let latin = Regex::new(r"[a-zA-Z][a-zA-Z_-]*").unwrap();
    let head_boundary = Regex::new(r"[\s(（]").unwrap();
    let mut v: Vec<String> = Vec::new();
    let mut seen = HashSet::new();
    for line in text.lines().filter(|l| row.is_match(l)) {
        // 先頭列だけを見る（`|` 分割の 2 番目のフィールド = term セル）。
        let Some(term_col) = line.split('|').nth(1) else {
            continue;
        };
        let cell = term_col.trim().trim_matches('`');
        // セル内も先頭トークンまで（最初の空白 or 開き括弧の手前）に絞る。
        let head = head_boundary.split(cell).next().unwrap_or(cell);
        for m in latin.find_iter(head) {
            let t = m.as_str();
            if t.chars().count() >= 2 && seen.insert(t.to_string()) {
                v.push(t.to_string());
            }
        }
    }
    v
}

/// 文書に用語表があるか（markdown table か HTML table/dl）を raw テキスト（構造保持）から見る。
/// 用語表の第 1 列に載る語の集合を返す ── 用語表そのものが無ければ空集合（(b) は不成立）。
fn glossary_terms(raw: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    // markdown table: `| 語 | ... |` 形の行群のうち、区切り行（|---|---|）の直後から始まる本体行。
    // 単純化: 全ての `|` 行の第 1 セルを候補に入れる（見出し行「語」自体は日本語のみで latin
    // マッチが無いため無害）。区切り行 `|---|` は第 1 セルが `-` のみなので latin 抽出で空。
    let md_row = Regex::new(r"^\s*\|").unwrap();
    let latin = Regex::new(r"[a-zA-Z][a-zA-Z_-]*").unwrap();
    for line in raw.lines().filter(|l| md_row.is_match(l)) {
        if let Some(first_cell) = line.split('|').nth(1) {
            for m in latin.find_iter(first_cell) {
                if m.as_str().chars().count() >= 2 {
                    out.insert(m.as_str().to_string());
                }
            }
        }
    }
    // HTML table: `<table>...<tr> ... <td class="mono">term(...)</td> ... </tr>` — 各 <tr> の
    // 最初の <td>/<th> セルだけを第 1 列として拾う。dl: `<dt>term</dt>` も同様に見出し語として扱う。
    let tr = Regex::new(r"(?is)<tr\b[^>]*>(.*?)</tr>").unwrap();
    let first_cell = Regex::new(r"(?is)<t[dh]\b[^>]*>(.*?)</t[dh]>").unwrap();
    for tr_cap in tr.captures_iter(raw) {
        if let Some(cell) = first_cell.captures(&tr_cap[1]) {
            for m in latin.find_iter(&cell[1]) {
                if m.as_str().chars().count() >= 2 {
                    out.insert(m.as_str().to_string());
                }
            }
        }
    }
    let dt = Regex::new(r"(?is)<dt\b[^>]*>(.*?)</dt>").unwrap();
    for cap in dt.captures_iter(raw) {
        for m in latin.find_iter(&cap[1]) {
            if m.as_str().chars().count() >= 2 {
                out.insert(m.as_str().to_string());
            }
        }
    }
    out
}

/// 語の周辺（同一行）に定義の徴候があるか。
///   - 直後の括弧に日本語文字を含む言い換え: `campaign（測定の実施計画）` / `campaign(測定の実施計画)`
///   - 直前の括弧に日本語文字を含む言い換え（逆順）: `（測定の実施計画）campaign`
///   - 同一行内に「とは」による定義文: `campaign とは…である` 等（語の後方に「とは」が近傍で続く）
fn has_definition_nearby(line: &str, term: &str) -> bool {
    let Some(pos) = line.find(term) else {
        return false;
    };
    let end = pos + term.len();
    let after = &line[end..];
    let before = &line[..pos];

    // 直後の括弧（全角/半角）に日本語文字を含む。
    if let Some(paren) = leading_paren_content(after)
        && contains_japanese(paren)
    {
        return true;
    }
    // 直前の括弧（全角/半角）に日本語文字を含む（逆順定義）。
    if let Some(paren) = trailing_paren_content(before)
        && contains_japanese(paren)
    {
        return true;
    }
    // 「とは」定義文: 語の直後 ~4 字以内に「とは」が現れる。
    let after_head: String = after.chars().take(6).collect();
    if after_head.contains("とは") {
        return true;
    }
    false
}

/// s の先頭が `（...）` か `(...)` なら中身を返す（先頭に空白は許容）。
fn leading_paren_content(s: &str) -> Option<&str> {
    let t = s.trim_start_matches(' ');
    if let Some(rest) = t.strip_prefix('（') {
        return rest.split('）').next();
    }
    if let Some(rest) = t.strip_prefix('(') {
        return rest.split(')').next();
    }
    None
}

/// s の末尾が `（...）` か `(...)` なら中身を返す（末尾に空白は許容）。
fn trailing_paren_content(s: &str) -> Option<&str> {
    let t = s.trim_end_matches(' ');
    if t.ends_with('）') {
        let start = t.rfind('（')?;
        return Some(&t[start + '（'.len_utf8()..t.len() - '）'.len_utf8()]);
    }
    if t.ends_with(')') {
        let start = t.rfind('(')?;
        return Some(&t[start + 1..t.len() - 1]);
    }
    None
}

fn contains_japanese(s: &str) -> bool {
    s.chars()
        .any(|c| matches!(c, '\u{3040}'..='\u{30FF}' | '\u{4E00}'..='\u{9FFF}'))
}

/// register=practice のときだけ呼ぶ想定。raw は構造保持テキスト（用語表判定用）、
/// clean は出現・定義徴候判定用（fence/inline code/URL 等を除いた地の文相当）。
/// terms は語彙表（load_terms）の見出し語一覧 — 呼び出し側が事前に読み込んで渡す。
pub fn scan(raw: &str, clean: &str, terms: &[String]) -> Vec<Finding> {
    if terms.is_empty() {
        return Vec::new();
    }
    let glossary = glossary_terms(raw);
    let mut out = Vec::new();
    // fence/inline code の除去は clean_lines が既に行っている（呼び出し側の契約）。
    for term in terms {
        if glossary.contains(term) {
            continue; // (b) 用語表に載っている — 合格
        }
        // 語境界（前後が latin 英数字でない）で出現を探す — 語の部分文字列一致を避ける
        // （「menu」が「menu-unrestricted」の一部として誤ヒットしない、の逆方向）。
        let boundary = Regex::new(&format!(
            r"(?:^|[^A-Za-z0-9_]){}(?:$|[^A-Za-z0-9_])",
            regex::escape(term)
        ))
        .unwrap();
        let mut first_hit: Option<usize> = None;
        for (i, line) in clean.lines().enumerate() {
            if boundary.is_match(line) {
                first_hit = Some(i + 1);
                break;
            }
        }
        let Some(line_no) = first_hit else {
            continue; // 出現なし
        };
        let line_text = clean.lines().nth(line_no - 1).unwrap_or("");
        if has_definition_nearby(line_text, term) {
            continue; // (a) 初出直近に定義の徴候がある — 合格
        }
        out.push(Finding {
            line: line_no,
            rule: "jargon-export",
            msg: format!(
                "term 「{term}」 from the internal vocabulary registry is used without a reader-facing definition — add a Japanese gloss in parentheses at first use, or add it to a glossary table this document defines",
            ),
        });
    }
    out
}

/// 等幅（monospace）の識別子文脈として扱う HTML 要素の**中身のテキスト**（tag を剥いだ生文字列）
/// の集合を raw から集める（第四波追加）。`<code>`/`<pre>` は prose::strip_html が既に「言及」
/// として同数改行で消す（fence/inline code と同型の既存規約）ので対象外 ── 本関数が拾うのは
/// それでは届かない残りのケース: `class` 属性に `mono` を含む任意要素
/// （`<td class="mono">campaign_h2_4arm</td>` のような、用語表以外の識別子セル・バッジ・
/// コードっぽい注記）。markdown 側は対応する記法が無い（`class` 属性は HTML 専有）ため常に空集合。
fn monospace_context_texts(raw: &str) -> Vec<String> {
    // regex クレートはバックリファレンス（`\1` で開始タグ名に閉じタグ名を揃える）を持たない
    // （strip_fences/code_pre と同じ制約）ので、開始タグの後、**最初の閉じタグ**（種類は問わない）
    // までを中身とみなす — mono セルは実測上ネストしたタグを持たない plain text（実測: td
    // class="mono" の中身は識別子文字列のみ）ため、この単純化で十分。
    let mono = Regex::new(
        r#"(?is)<[a-zA-Z][a-zA-Z0-9]*\b[^>]*\bclass\s*=\s*["'][^"']*\bmono\b[^"']*["'][^>]*>(.*?)</[a-zA-Z][a-zA-Z0-9]*\s*>"#,
    )
    .unwrap();
    let inner_tag = Regex::new(r"(?s)<[^>]*>").unwrap();
    mono.captures_iter(raw)
        .map(|c| inner_tag.replace_all(&c[1], "").trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// clean（出現判定用テキスト）から、等幅識別子文脈の中身と完全一致する語トークンを空白化する。
/// 「行全体をマスクする」のではなく「識別子の生文字列そのもの」を置換するので、同じ行に地の文の
/// 出現（識別子文脈の外）が同居していても、そちらは残って検査対象のままになる（実測:
/// `<td class="mono">campaign_h2_4arm / …</td><td>… campaign を並置した</td>` の
/// 隣接セルは識別子でなく地の文 — 同じ行でも区別が要る）。
fn mask_monospace_occurrences(clean: &str, mono_texts: &[String]) -> String {
    let mut out = clean.to_string();
    for t in mono_texts {
        if t.is_empty() {
            continue;
        }
        out = out.replace(t.as_str(), &" ".repeat(t.chars().count()));
    }
    out
}

/// register=consume のときだけ呼ぶ想定。合否の向きが scan と逆 ── 語彙表の語が地の文
/// （コードフェンス・インラインコード・等幅の識別子文脈を除く）に出現した時点で、それ自体が
/// advisory 違反になる。近傍の定義徴候（(a)）や文書内の用語表（(b)）はどちらも合格条件に
/// **ならない**（用語表を建てる解は「読者に書き手の語彙を習得させる」誤り ── 消費型読者には
/// 不出現こそが正しい合格条件、というのが本 register の存在理由そのもの）。
/// raw は等幅識別子文脈の抽出専用（strip_html 前の構造保持テキスト・scan と同じ契約）、
/// clean は出現判定用（呼び出し側が prose::clean_lines で作った散文相当・scan と同じ契約）。
pub fn scan_consume(raw: &str, clean: &str, terms: &[String]) -> Vec<Finding> {
    if terms.is_empty() {
        return Vec::new();
    }
    let mono_texts = monospace_context_texts(raw);
    let masked = mask_monospace_occurrences(clean, &mono_texts);
    let mut out = Vec::new();
    for term in terms {
        let boundary = Regex::new(&format!(
            r"(?:^|[^A-Za-z0-9_]){}(?:$|[^A-Za-z0-9_])",
            regex::escape(term)
        ))
        .unwrap();
        let mut first_hit: Option<usize> = None;
        for (i, line) in masked.lines().enumerate() {
            if boundary.is_match(line) {
                first_hit = Some(i + 1);
                break;
            }
        }
        let Some(line_no) = first_hit else {
            continue; // 出現なし（等幅識別子文脈のみの出現を含む）— 合格
        };
        out.push(Finding {
            line: line_no,
            rule: "jargon-export",
            msg: format!(
                "term 「{term}」 is not in the reader's vocabulary — don't define it, say it in the reader's words instead (e.g. campaign → 測定の実施計画)",
            ),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn terms() -> Vec<String> {
        vec!["campaign".into(), "menu".into(), "criterion".into()]
    }

    #[test]
    fn flags_undefined_jargon_export() {
        let text = "campaign を設計する。menu を選ぶ。criterion で評価する。";
        let hits = scan(text, text, &terms());
        let rules: Vec<&str> = hits.iter().map(|f| f.rule).collect();
        assert_eq!(rules, vec!["jargon-export"; 3]);
    }

    #[test]
    fn passes_when_parenthetical_gloss_follows_at_first_use() {
        let text = "campaign（測定の実施計画）を設計する。";
        assert!(scan(text, text, &terms()).is_empty());
    }

    #[test]
    fn passes_when_parenthetical_gloss_precedes_in_reverse_order() {
        let text = "（測定の実施計画）campaign を設計する。";
        assert!(scan(text, text, &terms()).is_empty());
    }

    #[test]
    fn passes_when_half_width_parens_used() {
        let text = "campaign(測定の実施計画)を設計する。";
        assert!(scan(text, text, &terms()).is_empty());
    }

    #[test]
    fn passes_when_towa_definition_sentence_present() {
        let text = "campaign とは測定の実施計画である。";
        assert!(scan(text, text, &terms()).is_empty());
    }

    #[test]
    fn does_not_pass_on_non_japanese_parenthetical() {
        // 括弧内が日本語でない（例: 記号だけ）は言い換えの徴候として認めない。
        let text = "campaign(E) を設計する。";
        let hits = scan(text, text, &terms());
        assert!(hits.iter().any(|f| f.msg.contains("campaign")));
    }

    #[test]
    fn passes_via_markdown_glossary_table_first_column() {
        let text =
            "| 語 | 定義 |\n|---|---|\n| campaign | 測定の実施計画 |\n\n本文で campaign を使う。";
        assert!(scan(text, text, &terms()).is_empty());
    }

    #[test]
    fn passes_via_html_table_first_column() {
        let html = "<table><thead><tr><th>語</th><th>定義</th></tr></thead><tbody><tr><td class=\"mono\">campaign(測定計画)</td><td>実施計画である。</td></tr></tbody></table><p>本文で campaign を使う。</p>";
        assert!(scan(html, html, &terms()).is_empty());
    }

    #[test]
    fn passes_via_html_dl_glossary() {
        let html = "<dl><dt>menu</dt><dd>測定設定の一覧</dd></dl><p>本文で menu を使う。</p>";
        assert!(scan(html, html, &terms()).is_empty());
    }

    #[test]
    fn only_first_occurrence_is_checked_per_term() {
        // 初出だけを見る（二回目以降に定義が無くても初出が合格なら advisory を出さない）。
        let text = "campaign（測定の実施計画）を設計する。次に campaign を評価する。";
        assert!(scan(text, text, &terms()).is_empty());
    }

    #[test]
    fn word_boundary_does_not_false_positive_on_substring() {
        // 「menu-unrestricted」中の menu は独立語ではないため、素朴な contains では誤検出しうる。
        // ここでは「menu」自体は出現していないことを確認する（境界一致の健全性）。
        let text = "この設計は menu-unrestricted である。";
        let hits = scan(text, text, &["menu".to_string()]);
        // menu-unrestricted 内の menu はハイフンで区切られており [^A-Za-z0-9_] 境界に一致するため
        // 出現として拾われる（意図: 複合語の構成要素としての露出も内輪語の輸出になりうる）。
        // ここでは「拾われた上で定義徴候が無ければ advisory になる」ことだけを固定する。
        assert!(hits.iter().any(|f| f.msg.contains("menu")));
    }

    #[test]
    fn empty_terms_list_never_fires() {
        assert!(scan("campaign menu criterion", "campaign menu criterion", &[]).is_empty());
    }

    #[test]
    fn load_terms_extracts_only_the_first_column_head_word() {
        let dir = std::env::temp_dir().join(format!("correo-jargon-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("taxonomy.md");
        std::fs::write(
            &path,
            "| term | home | gloss |\n|---|---|---|\n| `criterion alphabet A/c/L` | `src/bounds.jl` | some english gloss text |\n",
        )
        .unwrap();
        let terms = load_terms(&path);
        // 先頭トークン（criterion）だけを見出し語として拾う ── 「alphabet」は第 1 列内でも
        // 見出しに続く説明語であり、拾うと過検出（gloss と同じ地位の語まで語彙化する）になる。
        assert!(terms.contains(&"criterion".to_string()));
        assert!(!terms.iter().any(|t| t == "alphabet"));
        // home の path 断片や gloss 文中の語は候補に入らない。
        assert!(!terms.iter().any(|t| t == "bounds" || t == "src"));
        assert!(!terms.iter().any(|t| t == "some" || t == "english"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_terms_does_not_leak_embedded_standard_terms_from_explanatory_head() {
        // 回帰（2026-07-11・統制語彙表 taxonomy.md 実測で発覚）: 「セル全体」抽出だと
        // `thm:main (yield ⇔ tr(W_spec M⁻¹) L-optimal equivalence)` のような説明的な第 1 列から
        // yield・equivalence 等が拾われ、`任意 POVM (arbitrary POVM, menu-unrestricted)` からは
        // POVM のような広く定着した標準語（README が明示的に語彙表から除外する語）まで漏れた。
        // 先頭トークン限定でこれらを遮断する。
        let dir = std::env::temp_dir().join(format!("correo-jargon-test2-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("taxonomy.md");
        std::fs::write(
            &path,
            "| term | home | gloss |\n|---|---|---|\n\
             | `thm:main (yield equivalence)` | `x` | g |\n\
             | `任意 POVM (arbitrary POVM, menu-unrestricted)` | `x` | g |\n",
        )
        .unwrap();
        let terms = load_terms(&path);
        assert!(!terms.iter().any(|t| t == "yield" || t == "equivalence"));
        assert!(!terms.iter().any(|t| t == "POVM" || t == "menu"));
        std::fs::remove_dir_all(&dir).ok();
    }

    // ---- scan_consume（第四波・register=consume） ----

    #[test]
    fn consume_flags_bare_occurrence_with_no_definition() {
        let text = "campaign を設計する。";
        let hits = scan_consume(text, text, &terms());
        assert!(
            hits.iter().any(|f| f.msg.contains("campaign")),
            "定義の無い出現は consume でも違反のはず"
        );
    }

    #[test]
    fn consume_still_flags_occurrence_even_with_parenthetical_gloss() {
        // consume の中心契約: practice なら合格する近傍定義があっても、consume では
        // 出現そのものが違反 ── 「定義するのでなく言い換えよ」という処方の対象。
        let text = "campaign（測定の実施計画）を設計する。";
        let hits = scan_consume(text, text, &terms());
        assert!(
            hits.iter().any(|f| f.msg.contains("campaign")),
            "consume では近傍定義があっても違反のはず"
        );
    }

    #[test]
    fn consume_still_flags_occurrence_even_with_glossary_table() {
        // 用語表は practice の合格条件であって consume のそれではない（用語表は解でなく症状）。
        // 呼び出し契約どおり clean は prose::clean_lines 後（table 行は空行化される）を渡す。
        let text =
            "| 語 | 定義 |\n|---|---|\n| campaign | 測定の実施計画 |\n\n本文で campaign を使う。";
        let clean = crate::prose::clean_lines(text);
        let hits = scan_consume(text, &clean, &terms());
        assert!(
            hits.iter().any(|f| f.msg.contains("campaign")),
            "consume では用語表があっても地の文出現は違反のはず"
        );
    }

    #[test]
    fn consume_passes_when_term_never_appears() {
        let text = "この文書には対象語が一つも無い。";
        assert!(scan_consume(text, text, &terms()).is_empty());
    }

    #[test]
    fn consume_message_prescribes_rewording_not_defining() {
        let text = "campaign を設計する。";
        let hits = scan_consume(text, text, &terms());
        let h = hits.first().expect("finding が無い");
        assert!(
            h.msg.contains("reader's words") || h.msg.contains("reader's vocabulary"),
            "処方が読者の言葉での言い換えを指示していない: {}",
            h.msg
        );
    }

    #[test]
    fn consume_ignores_occurrence_inside_monospace_identifier_context() {
        // <td class="mono">campaign_h2_4arm</td> のような等幅の識別子文脈は言及であり地の文
        // ではない ── 語境界一致でも violation にしない。呼び出し契約どおり
        // raw=HTML 生ソース・clean=strip_html 後を渡す。
        let html = r#"<table><tr><td class="mono">campaign_id</td><td>説明</td></tr></table>"#;
        let clean = crate::prose::strip_html(html);
        let hits = scan_consume(html, &clean, &["campaign_id".to_string()]);
        assert!(
            hits.is_empty(),
            "等幅識別子文脈のみの出現が violation になった: {:?}",
            hits.iter().map(|f| &f.msg).collect::<Vec<_>>()
        );
    }

    #[test]
    fn consume_still_flags_same_term_when_it_also_appears_in_prose_outside_mono_cell() {
        // 実測パターン: 同じ行に mono セル（識別子）と地の文の両方が
        // 出現する場合、地の文側は依然として violation になる（行単位でなくトークン単位の除外）。
        let html = r#"<tr><td class="mono">campaign_h2_4arm</td><td>campaign を並置した</td></tr>"#;
        let clean = crate::prose::strip_html(html);
        let hits = scan_consume(html, &clean, &["campaign".to_string()]);
        assert!(
            hits.iter().any(|f| f.msg.contains("campaign")),
            "mono セル外の地の文出現が見逃された"
        );
    }

    #[test]
    fn consume_ignores_occurrence_inside_code_and_pre_like_other_detectors() {
        // <code>/<pre> は prose::strip_html が既に「言及」として消す既存規約 ── consume も
        // それに乗る（二重実装しない）。呼び出し契約どおり raw=HTML 生ソース・clean=strip_html
        // 後を渡す（pipeline.rs の jargon_findings と同じ経路）。
        let html = "<p>campaign を設計する。</p>\n<pre>campaign campaign campaign</pre>\n<p>ここには無い。</p>";
        let clean = crate::prose::strip_html(html);
        let hits = scan_consume(html, &clean, &["campaign".to_string()]);
        // 地の文の <p> 側（1 行目）の出現は依然として violation。<pre> 側（2 行目）からは
        // 拾わない ── strip_html が中身を同数改行で消すため出現自体が無い。
        assert_eq!(
            hits.iter()
                .find(|f| f.msg.contains("campaign"))
                .map(|f| f.line),
            Some(1)
        );
    }

    #[test]
    fn consume_ignores_occurrence_inside_markdown_inline_code() {
        // inline code の除去は prose::clean_lines が既に担当（呼び出し契約どおり clean は
        // clean_lines 後を渡す）── ここでは inline code だけの行（1 行目）に定義が
        // 無くても violation にならず、地の文の出現（2 行目）だけを拾うことを確認する。
        let text = "`campaign` は識別子の言及。\n地の文では campaign を設計する。";
        let clean = crate::prose::clean_lines(text);
        let hits = scan_consume(text, &clean, &["campaign".to_string()]);
        assert_eq!(
            hits.iter()
                .find(|f| f.msg.contains("campaign"))
                .map(|f| f.line),
            Some(2),
            "inline code 外（2 行目）の地の文出現から拾うはず: {:?}",
            hits.iter().map(|f| (f.line, &f.msg)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn consume_markdown_without_html_never_matches_monospace_regex() {
        // markdown 側には class 属性という記法が無いので monospace_context_texts は常に空集合
        // （無害に何もマッチしない）ことを回帰する。
        assert!(monospace_context_texts("# 見出し\n\ncampaign を設計する。").is_empty());
    }
}
