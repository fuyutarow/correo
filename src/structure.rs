// structure.rs — LLM 定型構造（layout slop）の検出。2026-07-09 に kinoshita.rs から分離。
// 分離の理由: この 4 規則は『理科系の作文技術』に無い（LLM 生成文の指紋 2 つ＋textlint-ja
// からの移植 2 つ）— 木下の名の下に置くのは出自の偽装だった。module 分割の原則は
// 「検出器 = 検査する性質 1 つ」で、出自は規則ごとに README の規則台帳が記録する。
// 全規則 advisory — layout には文脈の正当があり得るため、判定は judge/人へ回す。
use crate::report::{Severity, Violation};
use regex::Regex;

/// 構造 slop の全規則: bullet-template・colon-continuation（行単位）、
/// connector-pileup・opener-repetition（文単位）。抽出は prose.rs の単一 home に委譲。
pub fn scan(text: &str) -> Vec<Violation> {
    let mut v = Vec::new();
    let defenced = crate::prose::strip_fences(text);
    let sents = crate::prose::sentences(text);

    // 「- **見出し**: 説明」形式の箇条書きが 3 連続 — 生成文の指紋（革新性：/効率性： の列）。
    let bullet_tpl = Regex::new(r"^[ \t]*[-*+][ \t]\*\*[^*]+\*\*[:：]").unwrap();
    let mut tpl_run: Option<(usize, usize)> = None; // (開始行, 本数)
    let flush_tpl = |run: &mut Option<(usize, usize)>, v: &mut Vec<Violation>| {
        if let Some((line, n)) = run.take()
            && n >= 3
        {
            v.push(Violation {
                line,
                rule: "bullet-template",
                severity: Severity::Advisory,
                msg: format!(
                    "太字見出し＋コロンの箇条書きが {n} 連続 — LLM の定型 layout。散文か表を検討"
                ),
            });
        }
    };
    for (i, raw) in defenced.lines().enumerate() {
        if bullet_tpl.is_match(raw) {
            match tpl_run.as_mut() {
                Some((_, n)) => *n += 1,
                None => tpl_run = Some((i + 1, 1)),
            }
        } else {
            flush_tpl(&mut tpl_run, &mut v);
        }
    }
    flush_tpl(&mut tpl_run, &mut v);

    // 述語＋コロンで block（箇条書き/コード等）へ接続 — 英語 "Do the following:" の直訳調
    // （@textlint-ja no-ai-colon-continuation の Sudachi 不要 port・2026-07-09 harvest 採録）。
    // 名詞受け（使用方法:）は正当なので、コロン直前が「ひらがな＝述語的」の時だけ advisory。
    let pred_colon = Regex::new(r"[ぁ-ん][:：]\s*$").unwrap();
    let block_start = Regex::new(r"^[ \t]*(?:[-*+][ \t]|・|[0-9０-９]+[.．)]|```|>|\|)").unwrap();
    {
        let lines: Vec<&str> = defenced.lines().collect();
        for (i, raw) in lines.iter().enumerate() {
            if !pred_colon.is_match(raw.trim_end()) {
                continue;
            }
            if let Some(next) = lines[i + 1..].iter().find(|l| !l.trim().is_empty())
                && block_start.is_match(next)
            {
                v.push(Violation {
                    line: i + 1,
                    rule: "colon-continuation",
                    severity: Severity::Advisory,
                    msg: "述語＋コロンで箇条書きへ接続 — 英語の直訳調。「次の通り。」で切るか名詞で受ける"
                        .to_string(),
                });
            }
        }
    }

    // 文頭接続詞（また/さらに/そして…、）の連発 — LLM 生成文の指紋。3 文連続で advisory。
    let connector =
        Regex::new(r"^(また|さらに|そして|加えて|一方|なお|ただし|つまり|次に|まず)、").unwrap();
    let mut conn_run: Option<(usize, usize)> = None;
    let flush_conn = |run: &mut Option<(usize, usize)>, v: &mut Vec<Violation>| {
        if let Some((line, n)) = run.take()
            && n >= 3
        {
            v.push(Violation {
                line,
                rule: "connector-pileup",
                severity: Severity::Advisory,
                msg: format!(
                    "文頭接続詞が {n} 文連続（また/さらに/そして…）— 論理の接続を本文で書く"
                ),
            });
        }
    };

    // 同じ書き出しの文の連続（3 文以上）— もう一つの指紋。文頭接続詞（connector-pileup が
    // 担当）と重複しないよう、接続詞書き出しは対象外。
    let opener_of = |s: &str| -> String {
        s.chars()
            .take_while(|c| !matches!(c, '、' | '。' | '！' | '？'))
            .take(6)
            .collect()
    };
    let mut op_run: Option<(String, usize, usize)> = None; // (書き出し, 開始行, 本数)
    let flush_op = |run: &mut Option<(String, usize, usize)>, v: &mut Vec<Violation>| {
        if let Some((prev, start, n)) = run.take()
            && n >= 3
        {
            v.push(Violation {
                line: start,
                rule: "opener-repetition",
                severity: Severity::Advisory,
                msg: format!("同じ書き出し「{prev}…」が {n} 文連続 — 構文を変える"),
            });
        }
    };
    for (line, s) in &sents {
        if connector.is_match(s) {
            match conn_run.as_mut() {
                Some((_, n)) => *n += 1,
                None => conn_run = Some((*line, 1)),
            }
        } else {
            flush_conn(&mut conn_run, &mut v);
        }
        let o = opener_of(s);
        // bullet 行は対象外: 「- **Tier 1**…」「- **Tier 2**…」の列は正当な構造で、
        // 接頭が揃うのは当然（2026-07-09 dogfood で実測した FP class）。
        let is_bullet = matches!(s.chars().next(), Some('-' | '*' | '+' | '・' | '#'));
        let eligible = o.chars().count() >= 2 && !connector.is_match(s) && !is_bullet;
        match op_run.as_mut() {
            Some((prev, _, n)) if eligible && *prev == o => *n += 1,
            _ => {
                flush_op(&mut op_run, &mut v);
                if eligible {
                    op_run = Some((o, *line, 1));
                }
            }
        }
    }
    flush_conn(&mut conn_run, &mut v);
    flush_op(&mut op_run, &mut v);

    v.sort_by_key(|x| x.line);
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_bullet_template_run_of_three() {
        // 「- **見出し**: 説明」×3 連続 = LLM の定型 layout（革新性：/効率性： の列）。
        let bad = "- **革新性**: 高い。\n- **効率性**: 速い。\n- **拡張性**: 広い。";
        let hits: Vec<_> = scan(bad)
            .into_iter()
            .filter(|x| x.rule == "bullet-template")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(matches!(hits[0].severity, Severity::Advisory));
        let ok = "- **革新性**: 高い。\n- **効率性**: 速い。";
        assert!(scan(ok).iter().all(|x| x.rule != "bullet-template"));
    }

    #[test]
    fn flags_predicate_colon_before_block_but_allows_noun_colon() {
        // 英語 "Do the following:" の直訳調（述語＋コロン→箇条書き）。名詞受けは正当。
        let bad = "次の手順で設定を変更します:\n\n- 項目を開く。";
        assert!(scan(bad).iter().any(|x| x.rule == "colon-continuation"));
        let ok = "使用方法:\n\n- 項目を開く。";
        assert!(scan(ok).iter().all(|x| x.rule != "colon-continuation"));
    }

    #[test]
    fn connector_pileup_is_advisory_and_needs_three() {
        let bad = "また、測定を行った。さらに、解析を行った。そして、結論を得た。";
        let c: Vec<_> = scan(bad)
            .into_iter()
            .filter(|x| x.rule == "connector-pileup")
            .collect();
        assert_eq!(c.len(), 1);
        assert!(matches!(c[0].severity, Severity::Advisory));
        // 2 連続は不問（普通の文章にもある）。
        let ok = "また、測定を行った。さらに、解析を行った。";
        assert!(scan(ok).iter().all(|x| x.rule != "connector-pileup"));
    }

    #[test]
    fn flags_same_opener_three_sentences() {
        let bad = "本製品は、速い。本製品は、安い。本製品は、強い。";
        assert!(scan(bad).iter().any(|x| x.rule == "opener-repetition"));
        let ok = "本製品は、速い。本製品は、安い。";
        assert!(scan(ok).iter().all(|x| x.rule != "opener-repetition"));
    }
}
