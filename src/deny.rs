// deny.rs — 確定した造語の禁止辞書（prh residue の役割を correo に内蔵・2026-07-09）。
// judge の 3-way 裁定の第三バケツ: 自然→無視 / 使い続ける→allow / 確定造語→deny。
// deny は HARD（exit 1）— 一度「書き直す」と裁定した語の再侵入を機械が阻止する ratchet。
// fence 内・inline code 内は言及なので対象外（allow/coinage と同じ規約）。
use std::collections::HashMap;

/// 組み込みの slop 常套句（LLM の日本語が高頻度で混入させる定型 — 「実在するが使わない」の
/// 裁定を出荷時に同梱する。Vale が Google/Microsoft style を package で配るのと同型の
/// 「意見のある既定」）。個別解除は correo.toml の allow にその語を書く。
/// 選定基準は精度最優先: 人間の実用文にほぼ出現しない定型だけを載せ、
/// シームレス/徹底解説 のような「人間も使う」語は載せない（誤爆が信頼を殺す）。
/// pattern は語幹で持つ（活用差を吸収: 架け橋となる/なります）。
/// 2026-07-09 拡充: @textlint-ja/preset-ai-writing v1.7.0 の hype 辞書（25 語・MIT）と突合し、
/// 精度基準で 8 語を採録（和集合戦略 — 両辞書は部分交差する別集合だった）。不採録 13 語の理由:
/// 完全に/大幅に/最高の/完璧な/最先端の/次世代の/不可避の は人間の実用文の常用語
/// （guitarrapc の 200+ 記事実測で FP 上位の報告）、世界初/革命的/パラダイムシフト/民主化/究極の/
/// 驚異的 は正当な学術・事実主張に現れる。誤爆が信頼を殺す方針は変えない。
pub const BUILTIN: &[(&str, &str)] = &[
    ("可能性を解き放", "具体的に何ができるようになるかを書く"),
    ("架け橋とな", "具体的な機能・役割を書く"),
    ("世界へようこそ", "導入の常套句 — 内容から始める"),
    ("いかがでした", "ブログ定型の結び — 要点の再掲で締める"),
    ("ゲームチェンジャー", "何がどう変わるかを書く"),
    ("魔法のよう", "仕組みを書く"),
    ("潜在能力を引き出", "何がどう改善するかを数値で書く"),
    (
        "スーパーチャージ",
        "marketing の直訳 — 具体的な向上幅を書く",
    ),
    ("業界を再定義", "何が変わるかを書く"),
    ("フロンティアを開拓", "常套の比喩 — 対象領域を名指しする"),
    ("根本的に変革", "何がどう変わるかを書く"),
    ("驚嘆させ", "感情でなく事実・数値で示す"),
    ("未来を変える", "どの作業がどう変わるかを書く"),
    (
        "新たな基準を設定",
        "sets a new standard の直訳 — 基準の中身を書く",
    ),
];

/// BUILTIN を HashMap で返す（cfg.deny と併合し、allow で個別解除するのは呼び出し側）。
pub fn builtin() -> HashMap<String, String> {
    BUILTIN
        .iter()
        .map(|(w, s)| (w.to_string(), s.to_string()))
        .collect()
}

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

    #[test]
    fn builtin_slop_phrases_hit_with_stem_matching() {
        let b = builtin();
        // 語幹 match: 架け橋となります の活用形も捕まる。
        let hits = scan("本機能は両者の架け橋となります。", &b);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].1, "架け橋とな");
        // 常套句を含まない実用文は無音。
        assert!(scan("本機能は二つの系を接続する。", &b).is_empty());
    }
}
