// rhetoric.rs — 修辞密度（文体の指紋）の測定 emit。2026-07-09 較正 fleet（wf_0327d9d2）の実測で設計。
//
// 較正（per 100 文・語彙は策定済み 27 語幹）:
//   陽性 Bitter-Lesson.md:    ではなく 6.8 / —— 9.8 / 見出し副題 87% / メタファー 6.0
//   対照 correo README:      0 / 0 / 64% / 0
//   対照 人間の実務文書×2:   ≤0.2 / 0 / ≤43% / 0
//   人間の古典（青空実測）: ではなく 0.10–0.28/1000字 ≈ 0.5–1.4/100文 — 閾値 4.0 はこの 3〜8 倍上。
//
// 反証 fleet の裁定を設計に焼き込む:
//   (1) rate 単独で発火しない — 単独では全て正当な日本語技法。composite（2 指標以上の同時超過）のみ。
//   (2) 最低分母 30 文 — 短文書の率の暴発を封じる（density と同じ規約）。
//   (3) blockquote（> 行）除外 — 引用の巻き込みで誠実な評論ほど率が上がる倒錯を防ぐ。
//   (4) メタファーだけ独立発火 — 修理 action（語彙列挙→平叙語へ）を指せるため。対照 4 文書で 0。
//   (5) これは明晰さの欠陥でなく文体の指紋 — 全て advisory・測定値を emit し裁定は judge へ。
use crate::report::{Severity, Violation};
use regex::Regex;
use std::collections::HashSet;

/// 率の最低分母（文数）。これ未満の文書は計測不能として何も emit しない。
const MIN_SENTENCES: usize = 30;
/// composite の指標別閾値（較正表参照: 対照最大の 10 倍下・陽性の 6 割下に置く）。
const CONTRAST_RATE: f64 = 4.0;
const CONTRAST_MIN: usize = 6;
const DASH_RATE: f64 = 4.0;
const DASH_MIN: usize = 5;
const GLOSS_FRACTION: f64 = 0.8;
const GLOSS_MIN_HEADINGS: usize = 6;
const METAPHOR_RATE: f64 = 2.0;
const METAPHOR_MIN: usize = 3;

/// メタファー語彙表 loader。語彙は**機構でなく data**（judge 裁定の cache）— binary に
/// hardcode しない。正本は lexicons/metaphor-lex.tsv（採用 28 語幹＋棄却 35 語の裁定記録が
/// comment で同居）。形式: `語幹<TAB>書き換え指示`。file 不在 → 空 = metaphor 規則は沈黙
/// （coinage の辞書・corpus と同じ optional-data 規約）。
pub fn load_lexicon(path: &std::path::Path) -> Vec<(String, String)> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return vec![];
    };
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| match l.split_once('\t') {
            Some((stem, hint)) => (stem.trim().to_string(), hint.trim().to_string()),
            None => (l.to_string(), String::new()),
        })
        .collect()
}

/// 文書単位の修辞密度検査。metaphor_lex は外部語彙表（空なら metaphor 規則は沈黙）、
/// exempt（correo.toml の allow）で語幹を個別解除できる。
pub fn scan(
    text: &str,
    exempt: &HashSet<&str>,
    metaphor_lex: &[(String, String)],
) -> Vec<Violation> {
    let mut v = Vec::new();
    // blockquote は引用 — 地の文の指紋に合算しない（反証 guard #3）。
    let sents: Vec<(usize, String)> = crate::prose::sentences(text)
        .into_iter()
        .filter(|(_, s)| !s.trim_start().starts_with('>'))
        .collect();
    let n = sents.len();
    if n < MIN_SENTENCES {
        return v;
    }

    // 指標 1: 対立法「ではなく」。「ではない」への拡張は禁止（否定神学・弁証法を撃つ — 反証 guard #1）。
    let contrast_hits: Vec<usize> = sents
        .iter()
        .filter(|(_, s)| s.contains("ではなく"))
        .map(|(l, _)| *l)
        .collect();
    let contrast_rate = contrast_hits.len() as f64 * 100.0 / n as f64;
    let contrast_fire = contrast_hits.len() >= CONTRAST_MIN && contrast_rate >= CONTRAST_RATE;

    // 指標 2: 二倍ダッシュ「——」挿入。
    let dash_n: usize = sents.iter().map(|(_, s)| s.matches("——").count()).sum();
    let dash_rate = dash_n as f64 * 100.0 / n as f64;
    let dash_fire = dash_n >= DASH_MIN && dash_rate >= DASH_RATE;

    // 指標 3: 見出しの「— 副題」グロスの統一率（全見出しが同型 = template の指紋）。
    let heading = Regex::new(r"^#{2,} ").unwrap();
    let gloss = Regex::new(r"^#{2,} .+ — ").unwrap();
    let (mut ht, mut hg) = (0usize, 0usize);
    let defenced = crate::prose::strip_fences(text);
    for line in defenced.lines() {
        if heading.is_match(line) {
            ht += 1;
            if gloss.is_match(line) {
                hg += 1;
            }
        }
    }
    let gloss_fire = ht >= GLOSS_MIN_HEADINGS && (hg as f64 / ht as f64) >= GLOSS_FRACTION;

    // composite: 2 指標以上の同時超過でのみ 1 finding（単独の技法は正当 — 一様な多用が指紋）。
    let fired = [contrast_fire, dash_fire, gloss_fire]
        .iter()
        .filter(|x| **x)
        .count();
    if fired >= 2 {
        let line = contrast_hits.first().copied().unwrap_or(1);
        v.push(Violation {
            line,
            rule: "stylometric-uniformity",
            severity: Severity::Advisory,
            msg: format!(
                "stylometric uniformity: 「ではなく」 in {} sentences ({contrast_rate:.1}/100, calibrated cap {CONTRAST_RATE}), —— ×{dash_n} ({dash_rate:.1}/100), glossed headings {hg}/{ht} — each device is legitimate alone; the uniform overuse is the signature. Route to judge",
                contrast_hits.len()
            ),
        });
    }

    // 指標 4（独立発火）: 装飾メタファー密度。語彙を列挙する = 修理 action を指せる（codemix と同型）。
    let mention = Regex::new(r"「[^」]*」").unwrap(); // 言及（「架け橋」という常套句は…）は使用でない
    let mut counts: Vec<(&str, usize, usize, &str)> = Vec::new(); // (stem, count, first_line, hint)
    for (stem, hint) in metaphor_lex {
        if exempt.contains(stem.as_str()) {
            continue;
        }
        let (mut c, mut first) = (0usize, 0usize);
        for (l, s) in &sents {
            let clean = mention.replace_all(s, "");
            let k = clean.matches(stem).count();
            if k > 0 && first == 0 {
                first = *l;
            }
            c += k;
        }
        if c > 0 {
            counts.push((stem, c, first, hint));
        }
    }
    let total: usize = counts.iter().map(|x| x.1).sum();
    let meta_rate = total as f64 * 100.0 / n as f64;
    if total >= METAPHOR_MIN && meta_rate >= METAPHOR_RATE {
        counts.sort_by_key(|x| std::cmp::Reverse(x.1));
        let line = counts.iter().map(|x| x.2).min().unwrap_or(1);
        let vocab: Vec<String> = counts
            .iter()
            .map(|(s, c, _, _)| format!("{s}×{c}"))
            .collect();
        // 最頻語幹の書き換え指示を prompt として添える（語彙表の hint 列 = 修理 action）。
        let top_hint = counts
            .first()
            .filter(|x| !x.3.is_empty())
            .map(|x| format!("; top fix: {} → {}", x.0, x.3))
            .unwrap_or_default();
        v.push(Violation {
            line,
            rule: "metaphor-density",
            severity: Severity::Advisory,
            msg: format!(
                "decorative metaphors ×{total} ({meta_rate:.1}/100 sentences): {} — replace with plain terms, or add to allow if a deliberate central metaphor (judge decides){top_hint}",
                vocab.join("・")
            ),
        });
    }
    v.sort_by_key(|x| x.line);
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(n: usize) -> String {
        (0..n)
            .map(|i| format!("測定装置の較正を第{i}回目として実施した。"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn lex() -> Vec<(String, String)> {
        [
            ("発射台", "write 足がかり"),
            ("避難所", "write 退避先"),
            ("旅路", "write 過程"),
        ]
        .map(|(s, h)| (s.to_string(), h.to_string()))
        .to_vec()
    }

    #[test]
    fn composite_fires_only_when_two_indicators_exceed() {
        let ex = HashSet::new();
        // 対立法 8 文 + ダッシュ 6 本を 40 文の文書に一様散布 → composite 1 件。
        let mut doc = plain(32);
        for i in 0..8 {
            doc.push_str(&format!(
                "\nこれは手段ではなく——目的そのものが第{i}の争点である。"
            ));
        }
        let hits = scan(&doc, &ex, &[]);
        assert_eq!(
            hits.iter()
                .filter(|x| x.rule == "stylometric-uniformity")
                .count(),
            1
        );
        // 対立法だけ高率（ダッシュ・副題は無し）= FAQ/移行ガイド想定 → 発火しない。
        let mut faq = plain(32);
        for i in 0..8 {
            faq.push_str(&format!("\n第{i}項は同期ではなく非同期で動作する。"));
        }
        assert!(
            scan(&faq, &ex, &[]).is_empty(),
            "単独指標で composite が発火した"
        );
    }

    #[test]
    fn metaphor_density_lists_vocab_and_respects_allow() {
        let ex = HashSet::new();
        let mut doc = plain(30);
        doc.push_str("\n本基盤は開発の発射台である。発射台は知識の避難所である。");
        let hits = scan(&doc, &ex, &lex());
        let m: Vec<_> = hits
            .iter()
            .filter(|x| x.rule == "metaphor-density")
            .collect();
        assert_eq!(m.len(), 1);
        assert!(m[0].msg.contains("発射台×2"), "{}", m[0].msg);
        // 語彙表の hint 列が修理 prompt として message に載る。
        assert!(
            m[0].msg.contains("top fix: 発射台 → write 足がかり"),
            "{}",
            m[0].msg
        );
        // allow で語幹を解除すると閾値を割って沈黙する。空語彙表なら規則ごと沈黙する。
        let allowed: HashSet<&str> = ["発射台"].into();
        assert!(
            scan(&doc, &allowed, &lex())
                .iter()
                .all(|x| x.rule != "metaphor-density")
        );
        assert!(scan(&doc, &ex, &[]).is_empty(), "空語彙表で発火した");
    }

    #[test]
    fn short_documents_and_quotes_do_not_fire() {
        let ex = HashSet::new();
        // 30 文未満は計測不能 — どれだけ濃くても emit しない。
        let short = "手段ではなく——目的の発射台であり避難所への旅路を紡ぐ。".repeat(10);
        assert!(scan(&short, &ex, &lex()).is_empty());
        // 「」内の言及は使用に数えない。
        let mut doc = plain(30);
        doc.push_str("\n「発射台」「避難所」「旅路」という常套句を本稿は分析する。");
        assert!(
            scan(&doc, &ex, &lex())
                .iter()
                .all(|x| x.rule != "metaphor-density")
        );
    }
}
