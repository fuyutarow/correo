// deny.rs — 確定した造語の禁止辞書（prh residue の役割を correo に内蔵・2026-07-09）。
// judge の 3-way 裁定の第三バケツ: 自然→無視 / 使い続ける→allow / 確定造語→deny。
// deny は HARD（exit 1）— 一度「書き直す」と裁定した語の再侵入を機械が阻止する ratchet。
// fence 内・inline code 内は言及なので対象外（allow/coinage と同じ規約）。
// 組み込み辞書は 2 種の由来を持つ（BUILTIN=LLM 定型 slop 句、BUILTIN_COINAGE=個別裁定済みの
// 混種複合）— builtin() で統合して返し、呼び出し側（deny_findings）は区別しない。
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
    ("可能性を解き放", "state concretely what becomes possible"),
    ("架け橋とな", "name the concrete function or role"),
    ("世界へようこそ", "stock opener — start with the content"),
    (
        "いかがでした",
        "blog-closer boilerplate — end by restating the key points",
    ),
    ("ゲームチェンジャー", "state what changes and how"),
    ("魔法のよう", "explain the mechanism"),
    ("潜在能力を引き出", "state the improvement in numbers"),
    (
        "スーパーチャージ",
        "marketing calque — state the concrete gain",
    ),
    ("業界を再定義", "state what actually changes"),
    (
        "フロンティアを開拓",
        "stock metaphor — name the actual domain",
    ),
    ("根本的に変革", "state what changes and how"),
    ("驚嘆させ", "show facts and numbers, not awe"),
    ("未来を変える", "state which task changes and how"),
    (
        "新たな基準を設定",
        "calque of 'sets a new standard' — state the standard itself",
    ),
];

/// 裁定済みの混種複合（英語幹＋漢字接辞）の再侵入防止（2026-07-11・qoed 句レベル指摘）。
/// BUILTIN（LLM の定型 slop 句）とは判定の性質が別 — こちらは「造語として書き直す」と
/// 個別に裁定済みの語の ratchet であり、hype/marketing calque ではない。同じ deny の
/// HARD ratchet 機構（一度書き直すと決めた語の再侵入を機械が阻止する）を共有するため
/// builtin() で BUILTIN と統合するが、由来を区別するため配列は分ける。
pub const BUILTIN_COINAGE: &[(&str, &str)] = &[
    ("closed表", "決着済みの表 へ書き直す"),
    ("open表", "未決の表 へ書き直す"),
    ("copies数", "部数 へ書き直す"),
    ("software層", "ソフトウェア層 へ書き直す"),
    ("判定家族", "改名済みの旧称 — 現行名へ書き直す"),
];

/// w が BUILTIN_COINAGE 由来か（pipeline.rs が finding のメッセージ文言を由来ごとに
/// 出し分けるための判定 — 「LLM stock phrase」という文言は BUILTIN の hype/marketing calque
/// にしか当てはまらず、混種複合の裁定には別の文言が要る）。
pub fn is_coinage_term(w: &str) -> bool {
    BUILTIN_COINAGE.iter().any(|(term, _)| *term == w)
}

/// BUILTIN ∪ BUILTIN_COINAGE を HashMap で返す（cfg.deny と併合し、allow で個別解除するのは
/// 呼び出し側）。
pub fn builtin() -> HashMap<String, String> {
    BUILTIN
        .iter()
        .chain(BUILTIN_COINAGE.iter())
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

    #[test]
    fn builtin_coinage_terms_hit_with_rewrite_suggestion() {
        // qoed で「書き直す」と裁定済みの混種複合が builtin() に載り、deny の HARD ratchet で
        // 再侵入が阻止されること（2026-07-11・qoed 句レベル指摘）。
        let b = builtin();
        for (w, _) in BUILTIN_COINAGE {
            let text = format!("この{w}を確認する。");
            let hits = scan(&text, &b);
            assert_eq!(hits.len(), 1, "「{w}」が deny で拾われなかった");
            assert_eq!(hits[0].1, *w);
        }
    }

    #[test]
    fn builtin_coinage_mentions_in_code_are_not_hits() {
        let text = "`closed表` は言及。\n\n```\nopen表のコード例\n```";
        assert!(
            scan(text, &builtin()).is_empty(),
            "混種複合の言及が deny に落ちた"
        );
    }
}
