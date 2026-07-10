// prose.rs — markdown 散文抽出の単一 home（Biome「1-parse, multiple-passes」の correo 版）。
// 検出器（codemix / readability / 将来の calque・Tier2 proxy・Tier3 座標出力）は本 module の
// 散文単位を共有し、markdown の扱い（fence / 構造行 / inline code / URL / 段落 / bullet 行群 /
// 文分割）を各自で再実装しない。
// 抽出の経緯（2026-07-09）: fence pre-pass が codemix と readability に重複し、F2 修正を
// 二箇所へ適用する実害が出た — 共通変更点（CCP）は一 home へ。
//
// 行来歴の契約: 返す全単位は原文の 1-始まり行番号を持つ。除去（fence・構造行・空白のみ行）は
// 「同数改行の空行化」で行い、行番号を壊さない（codemix Para.line の「編集で直接開ける」を継承）。
use regex::Regex;

/// 散文単位＝段落 or 連続 bullet の行群。text 内の構造行は空行として位置を保持する。
pub struct ProseUnit {
    /// 開始行（1 始まり・原文基準）。
    pub line: usize,
    pub text: String,
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

/// マッチ全体を同数の改行へ置換する（strip_fences と同じ「同数改行置換」思想の共有 helper）。
fn blank_span(c: &regex::Captures) -> String {
    "\n".repeat(c[0].matches('\n').count())
}

/// HTML → 全検出器（readability/rhetoric/density/coinage 等）にそのまま通せる「散文だけ」の
/// テキストへ剥がす。**行数保存が絶対条件**（finding の行番号は HTML source の行を指す）。
///
/// 経緯（2026-07-11）: broad readership 向け成果物は HTML artifact だが、check にそれを通す
/// 経路が無く、利用者が雑な自前正規表現で tag を剥いで 2 検出器だけ回す羽目になった。素朴な
/// `<[^>]*>` → "" 置換は隣接セルの語を連結し（`<td>cell</td><td>固有</td>` → 「cell固有」）、
/// 大量の偽 coinage を生んだ実害がある — tag は空文字でなく**単一空白**に置換し、この連結を断つ。
///
/// 処理順: (1) script/style の span を中身ごと同数改行で消す（大小文字・複数行対応）。
/// (2) `<!-- … -->` コメントも同様に同数改行で消す。(3) 残る tag `<[^>]*>` は空白に置換 —
/// 単一行の tag は単一空白、属性折返しで複数行に跨る tag は改行だけ維持し非改行文字を空白化
/// する（同数改行置換だと tag 前後の語がなお連結し得るため、tag 自体は空白の「量」でなく
/// 「行数」だけを保存すればよい）。(4) 最小限の entity を decode する。
pub fn strip_html(text: &str) -> String {
    let script_style = Regex::new(r"(?is)<(?:script|style)\b[^>]*>.*?</(?:script|style)>").unwrap();
    let comment = Regex::new(r"(?s)<!--.*?-->").unwrap();
    let tag = Regex::new(r"(?s)<[^>]*>").unwrap();

    let no_script = script_style.replace_all(text, blank_span).into_owned();
    let no_comment = comment.replace_all(&no_script, blank_span).into_owned();
    let no_tag = tag
        .replace_all(&no_comment, |c: &regex::Captures| {
            let m = &c[0];
            if m.contains('\n') {
                m.chars()
                    .map(|ch| if ch == '\n' { '\n' } else { ' ' })
                    .collect::<String>()
            } else {
                " ".to_string()
            }
        })
        .into_owned();

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
fn clean_lines(text: &str) -> String {
    let defenced = strip_fences(text);
    let strip = Regex::new(r"`[^`]*`|https?://\S+|§\S+|R\d{4}_\d+|IF-\d").unwrap();
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
        .map(|(line, text, _)| ProseUnit { line, text })
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
        let html = "<table><tr><td>cell</td><td>固有</td></tr></table>";
        let out = strip_html(html);
        assert!(
            !out.contains("cell固有"),
            "tag 境界で別セルの語が連結してはいけない: {out:?}"
        );
        let collapsed = out.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            collapsed.contains("cell 固有"),
            "cell と 固有 の間に空白が入るべき: {out:?}"
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
}
