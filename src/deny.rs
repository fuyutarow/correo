// deny.rs — 確定した造語の禁止辞書（prh residue の役割を correo に内蔵・2026-07-09）。
// judge の 3-way 裁定の第三バケツ: 自然→無視 / 使い続ける→allow / 確定造語→deny。
// deny は HARD（exit 1）— 一度「書き直す」と裁定した語の再侵入を機械が阻止する ratchet。
// fence 内・inline code 内は言及なので対象外（allow/coinage と同じ規約）。
//
// 組み込みで持つのは BUILTIN（LLM 定型 slop 句）だけである。個別プロジェクトが自前で
// 「書き直す」と裁定した語（house-specific coinage）は correo 本体に焼き込まない ──
// 特定プロジェクトの裁定語を組み込みにすると、その語彙裁定が全リポジトリへ強制される
// （correo はドメイン非依存が設計原則・README「位置づけ」参照）。代わりに利用側が
// `correo.toml` の `[deny] vocabulary = "<path>"`（または `--deny-vocabulary <path>`）で
// 自前の禁止語彙表を渡す機構を持つ（load_vocabulary。rhetoric.rs の metaphor_lexicon と
// 同じ TSV data 契約 — 語彙は機構でなく data であり、binary に hardcode しない）。
use std::collections::{HashMap, HashSet};

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

/// BUILTIN を HashMap で返す（cfg.deny と併合し、allow で個別解除するのは呼び出し側）。
pub fn builtin() -> HashMap<String, String> {
    BUILTIN
        .iter()
        .map(|(w, s)| (w.to_string(), s.to_string()))
        .collect()
}

/// 利用側プロジェクトの禁止語彙表（`correo.toml` の `[deny] vocabulary` か
/// `--deny-vocabulary`）を読む。形式は rhetoric.rs の metaphor_lexicon と同じ TSV
/// data 契約: `語<TAB>書き直し案`（`#` 行と空行は無視・タブが無い行は書き直し案を
/// 空文字にして黙って受け付ける）。file が無い／読めなければ空 map（vocabulary 自体が
/// optional なので既定は無音）。deny の HARD ratchet 機構は変わらない — ここは語彙の
/// 出所を組み込みから利用側 file へ切り出すだけである。
pub fn load_vocabulary(path: &std::path::Path) -> HashMap<String, String> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return HashMap::new();
    };
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| match l.split_once('\t') {
            Some((word, rewrite)) => (word.trim().to_string(), rewrite.trim().to_string()),
            None => (l.to_string(), String::new()),
        })
        .collect()
}

/// 利用側プロジェクトの許容語彙表（`correo.toml` の `[codemix] allow-vocabulary` か
/// `--allow-vocabulary`）を読む。load_vocabulary（deny 側）と構造を並行させた対の機構 ──
/// flag 解析 → config 併合 → loader → runtime lookup の各段が同形（README「allow-vocabulary
/// 語彙表」参照）。deny と違い書き直し案の値は持たない（allow は「この語はこのまま許す」の
/// membership だけが意味を持つ）ので、値部分（タブ以降）があっても読み捨てて HashSet で返す。
/// 形式は同じ TSV 契約: `語`（または `語<TAB>備考`。`#` 行と空行は無視）。file が無い／
/// 読めなければ空集合（vocabulary 自体が optional なので既定は無音）。lookup は小文字化して
/// 比較する契約（codemix::domain_vocab・latin_token::scan と同じ）。
pub fn load_allow_vocabulary(path: &std::path::Path) -> HashSet<String> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return HashSet::new();
    };
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| {
            let word = l.split_once('\t').map_or(l, |(w, _)| w).trim();
            word.to_lowercase()
        })
        .filter(|w| !w.is_empty())
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

    fn write_temp_vocab(name: &str, content: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("correo-deny-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn load_vocabulary_parses_tab_separated_word_and_rewrite() {
        // 台帳体の実務文書から読者向け文書へ複写された際に確定した禁止語の再現例
        // （語 TAB 書き直し案の 2 語）。
        let path = write_temp_vocab(
            "vocab1.tsv",
            "closed表\t決着済みの表 へ書き直す\nopen表\t未決の表 へ書き直す\n",
        );
        let v = load_vocabulary(&path);
        assert_eq!(v.len(), 2);
        assert_eq!(
            v.get("closed表").map(String::as_str),
            Some("決着済みの表 へ書き直す")
        );
        assert_eq!(
            v.get("open表").map(String::as_str),
            Some("未決の表 へ書き直す")
        );
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn load_vocabulary_skips_comment_and_blank_lines() {
        let path = write_temp_vocab(
            "vocab2.tsv",
            "# comment line\n\ncopies数\t部数 へ書き直す\n  \n",
        );
        let v = load_vocabulary(&path);
        assert_eq!(v.len(), 1);
        assert!(v.contains_key("copies数"));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn load_vocabulary_accepts_tabless_line_with_empty_rewrite() {
        let path = write_temp_vocab("vocab3.tsv", "software層\n");
        let v = load_vocabulary(&path);
        assert_eq!(v.get("software層").map(String::as_str), Some(""));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn load_vocabulary_missing_file_returns_empty_map_not_error() {
        let v = load_vocabulary(std::path::Path::new("/nonexistent/correo-deny-vocab.tsv"));
        assert!(v.is_empty());
    }

    #[test]
    fn loaded_vocabulary_terms_hit_through_scan_like_builtin_terms() {
        // load_vocabulary が返す map は builtin() と同じ HashMap<String,String> 形なので
        // scan にそのまま渡せる（deny の HARD ratchet 機構は語彙の出所を問わない）。
        let path = write_temp_vocab(
            "vocab4.tsv",
            "判定家族\t改名済みの旧称 — 現行名へ書き直す\n",
        );
        let v = load_vocabulary(&path);
        let hits = scan("この判定家族を確認する。", &v);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].1, "判定家族");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn loaded_vocabulary_mentions_in_code_are_not_hits() {
        let path = write_temp_vocab("vocab5.tsv", "closed表\t書き直す\n");
        let v = load_vocabulary(&path);
        let text = "`closed表` は言及。\n\n```\nclosed表のコード例\n```";
        assert!(
            scan(text, &v).is_empty(),
            "語彙表由来の言及が deny に落ちた"
        );
        std::fs::remove_file(&path).ok();
    }

    // ---- load_allow_vocabulary（allow-vocabulary・deny-vocabulary と対の機構） ----

    #[test]
    fn load_allow_vocabulary_parses_bare_words_lowercased() {
        let path = write_temp_vocab("allow1.tsv", "Baseline\nMenu\n");
        let v = load_allow_vocabulary(&path);
        assert_eq!(v.len(), 2);
        assert!(v.contains("baseline"));
        assert!(v.contains("menu"));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn load_allow_vocabulary_ignores_value_after_tab() {
        // deny と同じ TSV 形（語<TAB>備考）を受理するが、値は読み捨てて membership だけ使う。
        let path = write_temp_vocab("allow2.tsv", "baseline\thouse-specific term\n");
        let v = load_allow_vocabulary(&path);
        assert_eq!(v.len(), 1);
        assert!(v.contains("baseline"));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn load_allow_vocabulary_skips_comment_and_blank_lines() {
        let path = write_temp_vocab("allow3.tsv", "# comment\n\nmenu\n  \n");
        let v = load_allow_vocabulary(&path);
        assert_eq!(v.len(), 1);
        assert!(v.contains("menu"));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn load_allow_vocabulary_missing_file_returns_empty_set_not_error() {
        let v = load_allow_vocabulary(std::path::Path::new("/nonexistent/correo-allow-vocab.tsv"));
        assert!(v.is_empty());
    }
}
