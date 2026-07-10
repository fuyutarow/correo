// prose.rs — markdown 散文抽出の単一 home（Biome「1-parse, multiple-passes」の correo 版）。
// 検出器（codemix / readability / 将来の calque・Tier2 proxy・Tier3 座標出力）は本 module の
// 散文単位を共有し、markdown の扱い（fence / 構造行 / inline code / URL / 段落 / bullet 行群 /
// 文分割）を各自で再実装しない。
// 抽出の経緯（2026-07-09）: fence pre-pass が codemix と readability に重複し、F2 修正を
// 二箇所へ適用する実害が出た — 共通変更点（CCP）は一 home へ。
//
// 行来歴の契約: 返す全単位は原文の 1-始まり行番号を持つ。除去（fence・構造行・空白のみ行）は
// 「同数改行の空行化」で行い、行番号を壊さない（codemix Para.line の「編集で直接開ける」を継承）。
use lol_html::html_content::ContentType;
use lol_html::{HtmlRewriter, Settings, comments, element, end_tag, text};
use regex::Regex;
use std::rc::Rc;

/// 散文単位＝段落 or 連続 bullet の行群。text 内の構造行は空行として位置を保持する。
pub struct ProseUnit {
    /// 開始行（1 始まり・原文基準）。
    pub line: usize,
    pub text: String,
    /// bullet 行群か（true なら text の各行が独立した項目・false なら 1 段落として折り返された
    /// 複数行が意味的に連続する文である ── completeness.rs が「1 行＝1 項目」判定を bullet
    /// block に限定するために公開・2026-07-11）。
    pub is_bullet: bool,
}

/// code fence を同数改行に置換して除去する（行番号不変）。fence 内はコード＝散文でない。
/// coinage の file/stdin モードも共有する（README のコード例内の語を造語として flag した
/// 実害の是正・2026-07-09 dogfood）。
pub fn strip_fences(text: &str) -> String {
    let fence = Regex::new(r"(?s)```.*?```").unwrap();
    fence
        .replace_all(text, |c: &regex::Captures| {
            "\n".repeat(c[0].matches('\n').count())
        })
        .into_owned()
}

/// タグ・コメント等の source 断片を「行数だけ保存する空白」へ変換する。埋め込み改行がある
/// 断片（属性折返しで複数行に跨る tag）は改行だけ残し非改行文字を空白化、単一行の断片は
/// 単一空白 1 個にする（strip_html の行数保存契約の核）。block 要素の単一行タグは句点相当の
/// 境界記号「。」を単一空白で挟んで返す ── 同一物理行に並ぶ複数 block（表セル等）が 1 文として
/// 誤って連結されるのを防ぐ（inline 要素は従来通り単一空白のみ・2026-07-11 精密化）。
fn blank_tag_boundary(raw: &str, is_block: bool) -> String {
    if raw.contains('\n') {
        raw.chars()
            .map(|ch| if ch == '\n' { '\n' } else { ' ' })
            .collect()
    } else if is_block {
        " 。 ".to_string()
    } else {
        " ".to_string()
    }
}

/// 散文単位の境界として扱う block 要素名（小文字）。この tag の開始/終了は「別の散文単位」の
/// 合図であり、テキストをまたいで連結してはいけない（`<td>cell</td><td>固有</td>` を
/// 「cell固有」と読ませない、という単一空白 tag 置換の初版の狙いを、同一物理行に複数 block が
/// 並ぶ表・リストでも保つための精密化・2026-07-11）。
const BLOCK_TAGS: &[&str] = &[
    "p",
    "li",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "td",
    "th",
    "tr",
    "div",
    "section",
    "table",
    "thead",
    "tbody",
    "ul",
    "ol",
    "footer",
    "header",
    "article",
    "blockquote",
    "br",
    "hr",
];

/// HTML → 全検出器（readability/rhetoric/density/coinage 等）にそのまま通せる「散文だけ」の
/// テキストへ剥がす。**行数保存が絶対条件**（finding の行番号は HTML source の行を指す）。
///
/// 経緯（2026-07-11）: broad readership 向け成果物は HTML artifact だが、check にそれを通す
/// 経路が無く、利用者が雑な自前正規表現で tag を剥いで 2 検出器だけ回す羽目になった。素朴な
/// `<[^>]*>` → "" 置換は隣接セルの語を連結し（`<td>cell</td><td>固有</td>` → 「cell固有」）、
/// 大量の偽 coinage を生んだ実害がある。
///
/// 精密化（2026-07-11・台帳体ダッシュボード HTML の table 実測で発覚）: 単一空白だけでは同一物理行に
/// 並ぶ複数 block 要素（`<tr><td>A</td><td>B</td></tr>` の各セル）が「1 つの文」として
/// sentences()/prose_units() に読まれ、文の資格検査（completeness）等が表データを地の文の
/// 断片と誤認する。block 要素の**境界**（開始/終了 tag）は散文単位の境界でもあるべき ──
/// 改行を増やせない契約（行数保存）の下では、境界に句点相当の終端記号を差し込んで
/// sentences() の文分割に「ここで文が切れる」と伝える。inline 要素（code/strong/a/span 等）は
/// 従来通り単一空白のみでテキストを連結する（描画と同じ字面を保つ）。
///
/// crate 選定の掃引（2026-07-11・実装は手書き regex トークナイズから `lol_html` へ差し替え）:
/// 属性値内の未エスケープ `>`（`<a href="a>b">` 等）は `<[^>]*>` が最初の `>` で止まり、
/// タグ境界より手前で誤って切れて断片が地の文に漏れる実害が実物で確認された
/// （`<a href="page?a=1>2">click</a>` → 旧実装は ` 2">click  next` を出力）。本物の
/// HTML5 tokenizer（属性値の引用符を認識する）に置き換えて修正。候補掃引（lol_html /
/// html5ever+scraper / tl / htmlparser / nanohtml2text 等、crates.io 実地評価）の結果:
/// - DOM 構築系（html5ever 経由の scraper・select・ammonia）は Text node 連結では
///   属性折返しでタグ自身が消費した改行を再現できず行数保存に**自然には**適合しない
///   （実験で確認: `<p class="a"\n b="c">v</p>` を DOM 経由で愚直に text 連結すると
///   4 行 → 2 行に潰れる）。html5ever の低レベル TokenSink は line_number は返すが
///   バイト spanは返さず、結局 span を自前で再構築する必要があり regex より優れない。
/// - `tl` は zero-dependency で `HTMLTag::raw()`/`boundaries()` がバイト span を返すが、
///   (a) span は「タグ全体（開始〜子〜終了）」のみで開始/終了タグ単体の span が別途取れず
///   子の境界から差分で導出する必要があり実装が複雑、(b) 実装中に `boundaries()` が実際の
///   終了位置よりバイト数が短い（`raw()` の長さと不一致）既知でなさそうなズレを実験で発見
///   （`<a href="...">click</a>` で `boundaries()=(0,29)` だが `raw()` は 30 byte）。
///   最終 release も 2024-01 と約 2.5 年停滞 — 採らない。
/// - `htmlparser`（xmlparser 系の HTML tokenizer）は地の文に `]]>` が含まれるだけで
///   `Err(InvalidCharData)` を返して以降のトークン化を打ち切る、また root 要素の外側の
///   テキストで `Err(UnknownToken)` になることを実験で確認 — XML 的に厳格すぎて実運用の
///   雑な HTML に耐えない。
/// - `nanohtml2text`/`html2text` 系は「読みやすいプレーンテキスト化」が目的で意図的に
///   改行を潰す・re-flow する（実験で 11 行 → 3 行に潰れることを確認）— 設計目的が違う。
/// - `lol_html`（Cloudflare, streaming rewriter）は `StartTag`/`EndTag`/`Comment` の
///   各 token が `source_location().bytes()` で個別にバイト span を返し、属性値の引用符も
///   正しく認識する本物の HTML5 tokenizer。上記の壊れる例が両方直る。依存は transitive で
///   ~26 crate 増（CSS selector engine 一式を含み今回の用途では過剰気味）だが、finding の
///   行番号という証跡の正しさに関わる箇所であり、正しさを最優先して採用した
///   （依存重量は既知のコストとして許容）。block 境界の「。」挿入・code/pre 除去は lol_html
///   移行後もタグ replace 側で同じ規則を保つ（振る舞いは regex 版から不変・合流時に移植）。
///
/// 処理順: (1) script/style/code/pre の中身を同数改行で消す（script/style は HTML5 の raw
/// text element として tokenizer が正しく認識・タグ様の文字列が中に出ても誤爆しない。
/// code/pre の中身は言及なので同様に消す・fence の扱いと同型）。(2) `<!-- … -->` コメントも
/// 同数改行で消す（bogus comment を含む）。(3) 残る start/end tag: block 要素は単一空白＋
/// 句点相当の境界記号、inline 要素は単一空白のみ（複数行に跨る tag は改行だけ維持し
/// 非改行文字を空白化・行数保存が最優先）。(4) 最小限の entity を decode する。
pub fn strip_html(text: &str) -> String {
    // on_end_tag ハンドラは 'static 境界を要求される（終了タグは後続 chunk で判明するため）
    // ので、&str の借用でなく Rc<str> で source を共有する。
    let source: Rc<str> = Rc::from(text);
    let mut output = Vec::with_capacity(text.len());
    {
        let text_for_start = Rc::clone(&source);
        let text_for_end = Rc::clone(&source);
        let text_for_comment = Rc::clone(&source);

        let settings = Settings::new()
            .append_element_content_handler(element!("*", move |el| {
                let text_for_end = Rc::clone(&text_for_end);
                let tag_name = el.tag_name();
                let is_block = BLOCK_TAGS.contains(&tag_name.as_str());
                let st = el.start_tag();
                let range = st.source_location().bytes();
                st.replace(
                    &blank_tag_boundary(&text_for_start[range], is_block),
                    ContentType::Text,
                );

                // void/self-closing 要素（img・br・hr 等）は終了タグを持たず
                // on_end_tag は can_have_content() が false だと Err を返すのでガードする。
                if el.can_have_content() {
                    el.on_end_tag(end_tag!(move |end| {
                        let range = end.source_location().bytes();
                        end.replace(
                            &blank_tag_boundary(&text_for_end[range], is_block),
                            ContentType::Text,
                        );
                        Ok(())
                    }))?;
                }
                Ok(())
            }))
            // script/style/code/pre の中身は散文でない — 同数改行のまま消す（前者は raw text
            // element・後者は fence と同型の「言及」除外）。
            .append_element_content_handler(text!("script, style, code, pre", |t| {
                let n = t.as_str().matches('\n').count();
                t.replace(&"\n".repeat(n), ContentType::Text);
                Ok(())
            }))
            // コメント（bogus comment 含む）はタグ delimiter ごと同数改行で消す。
            .append_element_content_handler(comments!("*", move |c| {
                let range = c.source_location().bytes();
                let raw = &text_for_comment[range];
                let n = raw.matches('\n').count();
                c.replace(&"\n".repeat(n), ContentType::Text);
                Ok(())
            }));

        let mut rewriter = HtmlRewriter::new(settings, |chunk: &[u8]| {
            output.extend_from_slice(chunk);
        });
        // rewrite_str 相当を手動で行う: write() は不正な状態にはならず、万一
        // (メモリ上限超過等の) rewriting error が出ても入力をそのまま返す安全側に倒す。
        if rewriter.write(text.as_bytes()).is_err() || rewriter.end().is_err() {
            return text.to_string();
        }
    }
    let no_tag = match String::from_utf8(output) {
        Ok(s) => s,
        Err(_) => return text.to_string(),
    };

    // &amp; は最後に decode する（&amp;lt; のような二重 escape を &lt; → < と誤って
    // 二段 decode しないため）。
    no_tag
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
}

/// 全文 → 行構造を保った「散文だけの」テキスト。
/// fence は同数改行置換で除去・空白のみ行（space/tab/全角）は空行化・表 `|` / 見出し `#` 行は
/// 空行化（位置保持）・inline code / URL / §ref / ledger id は文中から strip。
/// pub化（2026-07-11）: notation.rs が表/見出し/inline code 除外を再実装せず共有するため
/// （CCP は一 home へ、という本 module 冒頭の原則）。
pub fn clean_lines(text: &str) -> String {
    let defenced = strip_fences(text);
    // §ref は `\S+` の貪欲マッチが直後の閉じ括弧・句点まで飲み込む事故があった
    // （`(評論 §2)。` → 「)。」まで消えて文末が消失・2026-07-11・completeness 実装で発覚）。
    // §番号本体（数字・章番号のドット区切り）だけを対象にし、和文の閉じ括弧・句読点は
    // 飲み込まない（`\S` は非空白なら何でも喰うため、和文記号を明示的に除外境界にする）。
    let strip = Regex::new(
        r"`[^`]*`|https?://\S+|§[\p{Han}\p{Katakana}\p{Hiragana}0-9A-Za-z.\-]+|R\d{4}_\d+|IF-\d",
    )
    .unwrap();
    let mut out = String::new();
    for (i, raw) in defenced.lines().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let t = raw.trim_start();
        if t.starts_with('|') || t.starts_with('#') {
            continue; // 構造行 → 空行（散文でないが行位置は保つ）
        }
        let s = strip.replace_all(raw, "");
        if s.trim().is_empty() {
            continue; // 空白のみ行 → 空行
        }
        out.push_str(s.trim_end());
    }
    out
}

/// 全文 → 散文単位（段落 / bullet 行群）。
/// 連続する bullet 段落は 1 行群へ併合する（F1(b): 1 bullet ≒ 1 短段落は codemix の 40字床を
/// 割り、LLM slop の主戦場である箇条書きが不可視だった）。完全空 segment（4+ 連続空行・fence 跡）
/// は連鎖を切る。地の文段落とは併合しない。
pub fn prose_units(text: &str) -> Vec<ProseUnit> {
    let cleaned = clean_lines(text);
    let bullet = Regex::new(r"^(?:[-*+][ \t]|・|[0-9０-９]+[.．)])").unwrap();
    let is_bullet_block = |seg: &str| {
        let mut any = false;
        for l in seg.lines() {
            let t = l.trim_start();
            if t.is_empty() {
                continue;
            }
            if !bullet.is_match(t) {
                return false;
            }
            any = true;
        }
        any
    };
    let mut units: Vec<(usize, String, bool)> = Vec::new(); // (開始行, 本文, bullet 行群か)
    let mut line = 1usize;
    for para in cleaned.split("\n\n") {
        // 3 連続以上の改行では segment が先頭 "\n" を抱える — その分を進めて開始行が
        // 空行でなく本文行を指すようにする（off-by-one 是正）。
        let leading = para.len() - para.trim_start_matches('\n').len();
        let start_line = line + leading;
        line += para.matches('\n').count() + 2; // 段落内の改行 ＋ 区切りの "\n\n"
        if para.trim().is_empty() {
            if let Some(last) = units.last_mut() {
                last.2 = false; // 空 segment は bullet 連鎖を切る
            }
            continue;
        }
        let b = is_bullet_block(para);
        match units.last_mut() {
            Some((_, text, true)) if b => {
                text.push('\n');
                text.push_str(para);
            }
            _ => units.push((start_line, para.to_string(), b)),
        }
    }
    units
        .into_iter()
        .map(|(line, text, is_bullet)| ProseUnit {
            line,
            text,
            is_bullet,
        })
        .collect()
}

/// 全文 → 文（。！？ 区切り）。(開始行, 本文) を返す。行跨ぎの一文は接続する。
/// 空行・構造行（空行化済み）で接続を切る — 表や見出しを跨いで一文にしない
/// （旧 readability 実装は構造行で切ると註記しながら実際は接続していた comment-code 乖離の是正）。
pub fn sentences(text: &str) -> Vec<(usize, String)> {
    let cleaned = clean_lines(text);
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut buf_start = 0usize;
    let flush = |buf: &mut String, start: usize, out: &mut Vec<(usize, String)>| {
        let s = buf.trim();
        if !s.is_empty() {
            out.push((start, s.to_string()));
        }
        buf.clear();
    };
    for (i, raw) in cleaned.lines().enumerate() {
        let lineno = i + 1;
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            // 段落境界。文末記号なしで終わった段落は 1 文として flush（断片は後段の閾値が
            // ほぼ反応しないので害がない）。
            flush(&mut buf, buf_start, &mut out);
            continue;
        }
        if buf.is_empty() {
            buf_start = lineno;
        }
        for ch in trimmed.chars() {
            buf.push(ch);
            if matches!(ch, '。' | '！' | '？') {
                let s = std::mem::take(&mut buf);
                out.push((buf_start, s.trim().to_string()));
                buf_start = lineno; // 同一行の続きの文はこの行から
            }
        }
    }
    flush(&mut buf, buf_start, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const HTML_FIXTURE: &str = "<!DOCTYPE html>\n<html>\n<head>\n<style>\n  body { color: red; }\n  .warn { color: orange; }\n</style>\n<script>\n  function greet(name) {\n    console.log(name);\n  }\n</script>\n</head>\n<body>\n<!-- これは\n   複数行コメント -->\n<p>本文はここにある。</p>\n</body>\n</html>";

    #[test]
    fn clean_lines_section_ref_strip_does_not_swallow_trailing_punctuation() {
        // 回帰（2026-07-11・R2607_026 実測で発覚）: §\S+ の貪欲マッチが `§2)。` の閉じ括弧と
        // 句点まで飲み込み、completeness が文末消失を未終端と誤検出した。§番号本体だけを対象にし、
        // 直後の和文閉じ括弧・句点は残す。
        let text = "提案設計の最適性を機械検査できること(評論 §2)。";
        let cleaned = clean_lines(text);
        assert!(
            cleaned.ends_with('。'),
            "§ref 除去が文末の句点を飲み込んだ: {cleaned:?}"
        );
        // §番号本体は引き続き除去される（ledger id の strip 契約は不変）。
        assert!(!cleaned.contains("§2"), "§ref 本体が残った: {cleaned:?}");
    }

    #[test]
    fn strip_html_preserves_line_count_with_multiline_script_style_comment() {
        // finding の行番号が HTML source の行を指すため、行数保存は絶対条件。
        let out = strip_html(HTML_FIXTURE);
        assert_eq!(
            out.lines().count(),
            HTML_FIXTURE.lines().count(),
            "strip_html は入力と行数が一致しなければならない"
        );
    }

    #[test]
    fn strip_html_does_not_concatenate_adjacent_cells() {
        // tag を単一空白でなく空文字へ置換すると `<td>cell</td><td>固有</td>` が
        // 「cell固有」という偽複合語に化ける — 本 feature の存在理由そのもの。
        // 精密化（2026-07-11）: td は block 要素なので境界に句点相当の終端記号も入り、
        // 「cell」と「固有」は別文（別散文単位）として分かれる ── 語の連結防止に加えて
        // sentences()/prose_units() が同一物理行の複数セルを 1 文と誤読しない、より強い保証。
        let html = "<table><tr><td>cell</td><td>固有</td></tr></table>";
        let out = strip_html(html);
        assert!(
            !out.contains("cell固有"),
            "tag 境界で別セルの語が連結してはいけない: {out:?}"
        );
        let collapsed = out.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            collapsed.contains("cell")
                && collapsed.contains("固有")
                && !collapsed.contains("cell固有"),
            "cell と 固有 の間に空白（と block 境界）が入るべき: {out:?}"
        );
        // sentences() から見て cell と 固有 が別文であること（td 境界＝散文単位境界の本体保証）。
        let sents = sentences(&out);
        let joined_any_sentence = sents
            .iter()
            .any(|(_, s)| s.contains("cell") && s.contains("固有"));
        assert!(
            !joined_any_sentence,
            "cell と 固有 が同一文として結合された: {sents:?}"
        );
    }

    #[test]
    fn strip_html_decodes_minimal_entities() {
        assert_eq!(strip_html("A &amp; B&nbsp;C"), "A & B C");
        assert_eq!(
            strip_html("&lt;tag&gt; &quot;q&quot; &apos;a&apos;&#39;"),
            "<tag> \"q\" 'a''"
        );
    }

    #[test]
    fn strip_html_blanks_script_and_style_content() {
        let out = strip_html(HTML_FIXTURE);
        assert!(
            !out.contains("color: red"),
            "style の中身が残っている: {out:?}"
        );
        assert!(
            !out.contains("console.log"),
            "script の中身が残っている: {out:?}"
        );
        assert!(
            !out.contains("複数行コメント"),
            "comment の中身が残っている: {out:?}"
        );
        assert!(
            out.contains("本文はここにある"),
            "地の文まで消してはいけない: {out:?}"
        );
    }

    #[test]
    fn strip_html_preserves_lines_across_wrapped_attribute_tag() {
        // 属性折返しで複数行に跨る tag（開始 `<p ...` から `...>` まで 3 行）でも行数を保つ。
        let html = "<p class=\"a\"\n   data-x=\"b\"\n   id=\"c\">value</p>\nnext line";
        let out = strip_html(html);
        assert_eq!(out.lines().count(), html.lines().count());
        assert!(out.contains("value"));
        assert!(out.contains("next line"));
    }

    #[test]
    fn strip_html_concatenates_inline_elements_without_extra_boundary() {
        // inline 要素（code/strong/a 等）はテキストをそのまま連結する（空白を勝手に足さない・
        // 落とさない・描画と同じ字面）── block 境界の「。」挿入はここに適用してはいけない。
        let html = "<p>設計対象<strong>は</strong>下流に渡る<code>x</code>実験である。</p>";
        let out = strip_html(html);
        assert!(
            out.contains("設計対象 は 下流に渡る"),
            "inline 要素境界で語が壊れた: {out:?}"
        );
        // p タグ自体は block だが、その内側の inline 要素は文を割ってはいけない。
        let sents = sentences(&out);
        assert!(
            sents.iter().any(|(_, s)| s.contains("設計対象")
                && s.contains("下流に渡る")
                && s.contains("実験である")),
            "inline 要素が文を分断した: {sents:?}"
        );
    }

    #[test]
    fn strip_html_excludes_code_and_pre_content_as_mention() {
        // code/pre の中身は言及（mention）── fence の扱いと同型で検査対象から除く。
        let html = "<p>設定は <code>correo.toml</code> に書く。</p><pre>fn main() { 造語だらけ・関数群 }</pre>";
        let out = strip_html(html);
        assert!(
            !out.contains("correo.toml"),
            "code 内の言及が残った（検査対象になってしまう）: {out:?}"
        );
        assert!(!out.contains("造語だらけ"), "pre 内の言及が残った: {out:?}");
        assert!(
            out.contains("設定は"),
            "地の文まで消してはいけない: {out:?}"
        );
    }

    #[test]
    fn strip_html_separates_same_line_table_cells_into_distinct_sentences() {
        // 台帳体ダッシュボード HTML の実測: <tr><td>A</td><td>B</td></tr> が同一物理行に並ぶ表は、
        // 単一空白だけでは 1 文として誤読される（completeness/codemix 等が表データを地の文の
        // 断片と誤認する実害・2026-07-11）。td は block 要素なので境界に文の終端記号が入り、
        // 各セルは別の散文単位になるべき。
        let html =
            "<tr><td>step_jump_4</td><td>手順A から手順B への遷移</td><td>2.0〜4.7×</td></tr>";
        let out = strip_html(html);
        let sents = sentences(&out);
        assert!(
            sents
                .iter()
                .all(|(_, s)| !(s.contains("step_jump_4") && s.contains("手順A から"))),
            "同一物理行の隣接セルが 1 文に結合された: {sents:?}"
        );
    }

    #[test]
    fn strip_html_block_boundary_line_count_still_preserved() {
        // 境界記号の挿入（" 。 "）は文字を足すが改行は足さない ── 行数保存契約は不変。
        let html = "<table><tr><td>A</td><td>B</td></tr><tr><td>C</td><td>D</td></tr></table>";
        let out = strip_html(html);
        assert_eq!(out.lines().count(), html.lines().count());
    }

    // --- crate 掃引で実物確認した旧 regex 実装の穴（2026-07-11） ---
    // 手書き `<[^>]*>` は最初の '>' で tag を終端していたため、属性値内に未エスケープの
    // '>' があると tag 境界を誤検出し断片が地の文へ漏れていた。lol_html の本物の
    // HTML5 tokenizer は属性値の引用符を認識するため、この 2 件は直る。

    #[test]
    fn strip_html_handles_gt_inside_quoted_attribute_value() {
        // 旧 regex 実装は `<a href="page?a=1>2">click</a> next` を
        // ` 2">click  next` に壊していた（`>` で tag が誤って途中終端）。
        let html = "<a href=\"page?a=1>2\">click</a> next";
        let out = strip_html(html);
        assert_eq!(out.lines().count(), html.lines().count());
        assert!(
            !out.contains("\">") && !out.contains("2\">"),
            "属性値内の '>' で tag 境界が壊れ、断片が漏れてはいけない: {out:?}"
        );
        assert!(out.contains("click"), "地の文は残るべき: {out:?}");
        assert!(out.contains("next"), "地の文は残るべき: {out:?}");
    }

    #[test]
    fn strip_html_handles_gt_inside_void_element_attribute_value() {
        // 旧 regex 実装は `<img src="x.png?w=1>2" alt="a"><b>bold</b>` を
        // ` 2\" alt=\"a\"> bold ` に壊していた（void 要素でも同じ穴）。
        let html = "<img src=\"x.png?w=1>2\" alt=\"a\"><b>bold</b>";
        let out = strip_html(html);
        assert_eq!(out.lines().count(), html.lines().count());
        assert!(
            !out.contains("alt=\"a\""),
            "属性の残骸が地の文に漏れてはいけない: {out:?}"
        );
        assert!(out.contains("bold"), "地の文は残るべき: {out:?}");
    }

    #[test]
    fn strip_html_never_hard_fails_on_malformed_markup() {
        // 実 HTML には CDATA・不整合な閉じタグ・比較演算子入り script 等の「行儀の悪い」
        // 断片が混じる。crate 掃引で候補の一部（htmlparser）は同種の入力で Err を返し
        // トークン化を打ち切ることを確認した — strip_html はそれを許さず、常に
        // 行数保存された文字列を返す（壊れた断片が残ることはあっても、丸ごと失敗しない）。
        let cases = [
            "<p>before <![CDATA[ <fake-tag> still data ]]> after</p>",
            "<script>if (a < b && c > d) { x(); }</script><p>real text</p>",
            "<b><i>mismatched</b></i> tail",
        ];
        for html in cases {
            let out = strip_html(html);
            assert_eq!(
                out.lines().count(),
                html.lines().count(),
                "malformed input でも行数保存: {html:?} -> {out:?}"
            );
        }
    }
}
