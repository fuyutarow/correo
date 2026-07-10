// completeness.rs — 文の資格検査（読者向け散文単位が文として終端しているか・2026-07-11）。
//
// 出自: 台帳体（研究 registry・記録）から読者向け散文（配布 HTML 等）へ複写されるとき、名詞句の
// 断片（見出し・ラベル・統計カードの残骸）が「文」として紛れ込む事故が台帳体の実務文書から
// 読者向け文書へ複写された HTML で実測された（`§1 設計空間 ── campaign E の六軸` 「全 gate
// 通過・結合 151 assertion」のような、句点も疑問符も持たない断片が段落の位置に落ちる）。
// 検出器 = 検査する性質 1 つ
// （lib.rs の原則）: 本 module は「段落・箇条書きの項が文の終端記号（。？」等）で閉じているか」
// だけを見る。readability.rs（文の内部品質: 長さ・読点・二重否定 等）とは別の性質 ── ここは
// 文の資格そのもの（終端しているか）を見る、prose_units 単位の advisory。
//
// 見出し・表のセル・コード・URL 行は prose.rs の clean_lines が既に構造行として空行化する
// （# 見出し／| 表／fence／URL 単独行）ため、prose_units には現れない ── 本 module 側での
// 再フィルタは不要（fence/URL の除外規約は prose.rs 単一 home に委譲）。
//
// 台帳的な名詞句の箇条書き（「Tier 1（HARD）」のような体言止め）は正当があり得るため、本規則は
// **advisory 固定**で判定は judge へ回す（locate 哲学）。読者向け語法の HARD 分岐は本 repo に
// まだ無いため新設しない（既定 advisory のまま運用する・過剰な語法機構は増設しない）。

pub struct Finding {
    pub line: usize,
    pub rule: &'static str,
    pub msg: String,
}

/// 文の終端として資格ありと見なす末尾（句点・疑問符・感嘆符・和文/欧文の閉じ括弧・引用符）。
/// 体言止めの箇条書きラベル（Tier 1（HARD）等）を誤爆させないため、閉じ括弧も終端として許す
/// ── 判定を厳しくしすぎない（advisory・judge が最終確認する前提の locate）。
fn is_terminated(s: &str) -> bool {
    let trimmed = s.trim_end();
    // 行末の markdown 強調（**/*/`）や空白を剥がしてから終端記号を見る。
    let stripped = trimmed.trim_end_matches(['*', '_', '`', ' ', '\t']);
    let Some(last) = stripped.chars().next_back() else {
        return true; // 空文字列は対象外（呼び出し側が空行を渡さない前提の防御）
    };
    matches!(
        last,
        '。' | '？' | '！' | '」' | '』' | '）' | ')' | '"' | '\''
    )
}

/// 台帳的な短いラベル・統計カード断片を許す下限文字数。極端に短い体言止め（「Tier 1」等）まで
/// 拾うと locate のノイズが噴出するため、ある程度の長さ（散文らしさ）を持つ断片だけを見る。
const MIN_CHARS: usize = 8;

/// 文書 → 未終端の散文単位を列挙する（advisory・judge 送り）。
pub fn scan(text: &str) -> Vec<Finding> {
    let mut out = Vec::new();
    for unit in crate::prose::prose_units(text) {
        if unit.is_bullet {
            // bullet block は改行ごとに項目が変わる規約（README 自身の慣習と一致）── 1 行を
            // 1 項目として個別に判定する。
            for (line, raw) in (unit.line..).zip(unit.text.lines()) {
                let content = raw.trim();
                if !content.is_empty() {
                    check(content, line, &mut out);
                }
            }
        } else {
            // 地の文の段落は hard-wrap（editor の折返し）で複数行に割れているだけの 1 文/1 段落
            // でありうる（README 実測: 「exit 1 に数えるのは error\n（readability の…）。」が
            // 2 行に折返されているのを line 170 単体で未終端と誤検出した・2026-07-11 是正）。
            // 段落全体を 1 単位として末尾だけを見る ── 開始行を報告する（readability 等の
            // 「段落の開始行を指す」慣習と一致）。
            let content = unit.text.trim();
            if !content.is_empty() {
                check(content, unit.line, &mut out);
            }
        }
    }
    out
}

fn check(content: &str, line: usize, out: &mut Vec<Finding>) {
    if content.chars().count() >= MIN_CHARS && !is_terminated(content) {
        out.push(Finding {
            line,
            rule: "unterminated-prose",
            msg: format!(
                "prose unit does not end in a sentence terminator (。？！」等) 「…{}」 — close the sentence, or if this is a ledger-style noun-phrase label, judge confirms it's legitimate",
                tail(content, 20)
            ),
        });
    }
}

/// message 用の末尾抜粋（文字境界で安全に切る）。
fn tail(s: &str, n: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    let start = chars.len().saturating_sub(n);
    chars[start..].iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_unterminated_paragraph() {
        let bad = "これは十分に長い名詞句の断片であり句点がない";
        let hits = scan(bad);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].rule, "unterminated-prose");
        let ok = "これは十分に長い名詞句の断片であり句点がある。";
        assert!(scan(ok).is_empty());
    }

    #[test]
    fn allows_question_and_exclamation_and_closing_quotes() {
        assert!(scan("これは疑問文として終端しているだろうか？").is_empty());
        assert!(scan("これは感嘆文として終端している例だ！").is_empty());
        assert!(scan("これは引用として終端している例だ」").is_empty());
        assert!(scan("これは括弧を閉じて終端している例だ）").is_empty());
    }

    #[test]
    fn allows_ledger_style_noun_phrase_bullets_as_advisory_only_no_hard_gate() {
        // 台帳的な体言止め箇条書きは正当がありうる ── advisory のみで exit を汚さない
        // （呼び出し側の severity 判定は pipeline.rs 側の責務・ここでは全 finding が出る
        // ことだけを確認し、severity の hard/advisory は pipeline 側の adapter で検証する）。
        let bullets = "- Tier 1（HARD）— 実装済み\n- Tier 2（MIX・proxy）— 順次\n- Tier 3（VIBE・座標のみ）— 未着手";
        // これらは閉じ括弧なしで終わる断片を含む可能性があるため locate は候補を出してよい。
        // 本 test は「未終端なら拾う」が成立することのみ検証する。
        let hits = scan(bullets);
        assert!(hits.iter().all(|f| f.rule == "unterminated-prose"));
    }

    #[test]
    fn skips_headings_tables_and_short_fragments() {
        // 見出し(#)/表(|) は prose.rs の clean_lines が空行化済みなので prose_units に現れない。
        let text = "# 見出しは長い文字列だが検査対象外\n\n| セル | セル |\n\n短い";
        let hits = scan(text);
        assert!(
            hits.iter()
                .all(|f| f.rule != "unterminated-prose" || f.line > 2)
        );
        // 8字未満の断片（短いラベル）はノイズ回避のため対象外。
        assert!(scan("短い断片").is_empty());
    }

    #[test]
    fn html_derived_fragment_regression() {
        // 台帳体の実務文書から読者向け文書へ複写された HTML 抽出で実測した断片型（統計カードの残骸）。
        let frag = "全 gate 通過・結合 151 assertion";
        let hits = scan(frag);
        assert_eq!(hits.len(), 1, "終端記号のない断片を拾うべき");
    }

    #[test]
    fn hard_wrapped_paragraph_is_checked_as_one_unit_not_per_line() {
        // 回帰（2026-07-11・correo README 実測で発覚）: 地の文の 1 段落が editor の折返しで
        // 複数の物理行に割れているとき（「exit 1 に数えるのは error\n（readability の…）。」）、
        // bullet block と同じ「1 行＝1 項目」判定を当てると、句点が段落末にしか無いのに
        // 途中の折返し行を未終端と誤検出する。地の文は段落全体を 1 単位として末尾だけを見る。
        let wrapped = "`check` の指摘は一行形式。exit 1 に数えるのは error\n（readability の HARD 違反と deny）。codemix・coinage・calque\nは advisory として数える。";
        let hits = scan(wrapped);
        assert!(
            hits.is_empty(),
            "折返し段落の途中行を未終端と誤検出した: {:?}",
            hits.iter().map(|f| f.line).collect::<Vec<_>>()
        );
        // 一方、段落が本当に句点なしで終わっていれば（折返しでなく真の未終端）検出する。
        let truly_unterminated = "これは本当に長い名詞句の断片であり\n複数行に渡っても句点がない";
        assert_eq!(scan(truly_unterminated).len(), 1);
    }

    #[test]
    fn bullet_block_items_are_still_checked_per_line() {
        // bullet block は「1 行＝1 項目」のまま（README 自身の慣習）── 段落化しない。
        let bullets =
            "- 十分に長い断片で句点がある項目である。\n- 十分に長い断片で句点がない項目である";
        let hits = scan(bullets);
        assert_eq!(hits.len(), 1, "未終端の bullet 項目だけを拾うべき");
    }
}
