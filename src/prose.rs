// prose.rs — markdown 散文抽出の単一 home（Biome「1-parse, multiple-passes」の correo 版）。
// 検出器（codemix / kinoshita / 将来の calque・Tier2 proxy・Tier3 座標出力）は本 module の
// 散文単位を共有し、markdown の扱い（fence / 構造行 / inline code / URL / 段落 / bullet 行群 /
// 文分割）を各自で再実装しない。
// 抽出の経緯（2026-07-09）: fence pre-pass が codemix と kinoshita に重複し、F2 修正を
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
/// （旧 kinoshita 実装は構造行で切ると註記しながら実際は接続していた comment-code 乖離の是正）。
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
