// deny.rs — 確定した造語の禁止辞書（prh residue の役割を correo に内蔵・2026-07-09）。
// judge の 3-way 裁定の第三バケツ: 自然→無視 / 使い続ける→allow / 確定造語→deny。
// deny は HARD（exit 1）— 一度「書き直す」と裁定した語の再侵入を機械が阻止する ratchet。
// fence 内・inline code 内は言及なので対象外（allow/coinage と同じ規約）。
use std::collections::HashMap;

/// text から deny 語の出現を (行, 語, 書き直し案) で列挙する。
pub fn scan(text: &str, deny: &HashMap<String, String>) -> Vec<(usize, String, String)> {
    if deny.is_empty() {
        return vec![];
    }
    let defenced = crate::prose::strip_fences(text);
    let inline = regex::Regex::new(r"`[^`]*`").unwrap();
    let mut out = vec![];
    for (i, line) in defenced.lines().enumerate() {
        let clean = inline.replace_all(line, "");
        for (w, sugg) in deny {
            if clean.contains(w.as_str()) {
                out.push((i + 1, w.clone(), sugg.clone()));
            }
        }
    }
    out.sort(); // HashMap の順序は不定 — 出力を決定的に
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deny() -> HashMap<String, String> {
        HashMap::from([("機械床".to_string(), "書き直す".to_string())])
    }

    #[test]
    fn flags_denied_term_with_line_and_suggestion() {
        let hits = scan("一行目は普通。\n機械床を使う。", &deny());
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].0, 2);
        assert_eq!(hits[0].1, "機械床");
    }

    #[test]
    fn mentions_in_fence_and_inline_code_are_not_hits() {
        let text = "`機械床` は言及。\n\n```\n機械床のコード例\n```";
        assert!(scan(text, &deny()).is_empty(), "言及が deny に落ちた");
    }
}
