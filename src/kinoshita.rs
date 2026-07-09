// kinoshita.rs — 木下是雄『理科系の作文技術』HARD 層（機械が言い切れる床）の lint。
// Tier 1（2026-07-09 スコープ拡張・決定的実験「木下違反まみれ文書に correo 両検出器 PASS」の是正）:
//   文長・読点過多・ですます/である混在・慣用二重否定・ぼかし連発・指示語連鎖。
// 対象外（locate 哲学の分業・README roadmap 参照）:
//   Tier 2 = 係り受けが要る原則（逆茂木・主述近接）→ proxy 化して flag のみ（順次）。
//   Tier 3 = judge が要る原則（トピックセンテンス・事実と意見・スリカエ）→ LLM-judge へ座標を渡す。
// 検査単位は「文」（。！？ 区切り）。code fence・表・見出し・inline code は codemix と同じ思想で
// 地の文から除外する。閾値は運用床であって木下の数値ではない（木下は「短く切れ」としか言わない）。
use regex::Regex;
use std::fs;
use std::io::Read;

pub struct KinoshitaArgs {
    /// 一文の最大文字数（既定 100 — LP textlint 床と同値の運用床）
    pub max_sentence: usize,
    /// 一文の最大読点数（既定 4）
    pub max_ten: usize,
    pub advisory: bool,
    pub files: Vec<String>,
}

/// Hard = 機械判定が最終（blocking・exit に数える）。Advisory = 高精度 heuristic だが
/// 文脈で正当があり得る（報告のみ・judge/人の確認へ回す — MIX tier）。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Hard,
    Advisory,
}

pub struct Violation {
    pub line: usize,
    pub rule: &'static str,
    pub severity: Severity,
    pub msg: String,
}

/// Tier 1 の全規則を走らせ違反を列挙する（判定は全て機械的＝HARD。密度系は「連発/連鎖」の
/// 閾値で単発を許す — 単発のぼかしが妥当かは Tier 3 の judge 領分で、ここでは咎めない）。
pub fn scan(text: &str, max_sentence: usize, max_ten: usize) -> Vec<Violation> {
    // 慣用二重否定（否定の否定で読みを迂回させる決まり文句のみ。なければならない 等の
    // 標準的義務表現は二重否定ではないので対象外）。
    let dneg =
        Regex::new(r"なくはな|なくもな|ないことはな|ないとは(言|い)えな|ないわけではな").unwrap();
    // ぼかし（木下「言い切れ」の対偶側）。1 文 2 つ以上で「連発」。
    let hedge = Regex::new(
        r"と思われ|と考えられ|かもしれ|のではないか|ではないだろうか|気がする|感じがする|ように思|ように見え|とみられ",
    )
    .unwrap();
    // 指示語。1 文 3 つ以上で「連鎖」（この/その 2 回は日本語として普通なので許す）。
    let demons = Regex::new(r"これ|それ|あれ|この|その|あの|ここ|そこ").unwrap();
    // 文体: 敬体（ですます）と常体の文末。混在＝文書レベル違反。常体は「非敬体 ∧ ひらがな
    // 終わり」で近似する（する。/行った。/ない。等の終止形を覆い、体言止めは除外＝様式であって
    // 文体でないので数えない）。
    let polite =
        Regex::new(r"(です|ます|ません|でした|ました|ましょう|でしょう|ください)[。！？]$")
            .unwrap();
    let plain = Regex::new(r"[ぁ-ん][。！？]$").unwrap();
    // の連鎖（木下: 連続する「の」は 2 つまで）。この/その等の指示詞の の は連鎖に数えない。
    // 『…』引用タイトル内の の も数えない（『理科系の作文技術』の の は書名の一部 —
    // 2026-07-09 dogfood で実測した FP class。タイトルは 1 名詞として ◯ にマスクして測る）。
    let no_chain = Regex::new(r"(?:[^\s。、！？の]{1,12}の){3,}").unwrap();
    let title_span = Regex::new(r"『[^』]*』").unwrap();
    // 冗長表現（簡潔の原則・第8章）: することができる → できる。LLM 日本語の頻出 slop。
    let verbose = Regex::new(r"することが(でき|可能)").unwrap();
    // 文頭接続詞（また/さらに/そして…、）の連発 — LLM 生成文の指紋。3 文連続で advisory。
    let connector =
        Regex::new(r"^(また|さらに|そして|加えて|一方|なお|ただし|つまり|次に|まず)、").unwrap();
    // 7 連続以上の漢字（読みにくい漢語の塊・textlint max-kanji-continuous-len 相当）。
    // 分野の固有名（電子情報通信工学専攻）は正当なので advisory — 分割 or 固有名なら無視。
    let kanji_run = Regex::new(r"[\p{Han}々]{7,}").unwrap();
    // 実用文中の感嘆符（木下: 技術文書で ！ は避ける）。？ は問い掛けで正当があるので ！ のみ。
    let exclaim = Regex::new(r"[！!]").unwrap();

    // 文抽出（fence/構造行/inline strip・行跨ぎ接続）は prose.rs の単一 home に委譲（2026-07-09）。
    let sents = crate::prose::sentences(text);
    let mut v = Vec::new();
    let mut polite_lines: Vec<usize> = Vec::new();
    let mut plain_lines: Vec<usize> = Vec::new();
    // 文頭接続詞の連続 run（開始行, 本数）。3 本以上で advisory 1 件。
    let mut conn_run: Option<(usize, usize)> = None;
    fn flush_conn(run: &mut Option<(usize, usize)>, v: &mut Vec<Violation>) {
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
    }

    // 構造 slop（LLM の定型 layout・2026-07-09 「slop パターンは限定的」の議論より）:
    // 「- **見出し**: 説明」形式の箇条書きが 3 連続 — 生成文の指紋（革新性：/効率性： の列）。
    let bullet_tpl = Regex::new(r"^[ \t]*[-*+][ \t]\*\*[^*]+\*\*[:：]").unwrap();
    let defenced = crate::prose::strip_fences(text);
    let mut tpl_run: Option<(usize, usize)> = None; // (開始行, 本数)
    for (i, raw) in defenced.lines().enumerate() {
        if bullet_tpl.is_match(raw) {
            match tpl_run.as_mut() {
                Some((_, n)) => *n += 1,
                None => tpl_run = Some((i + 1, 1)),
            }
        } else {
            if let Some((line, n)) = tpl_run.take()
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
        }
    }
    if let Some((line, n)) = tpl_run.take()
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

    // 同じ書き出しの文の連続（3 文以上）— LLM 生成文のもう一つの指紋。文頭接続詞
    // （connector-pileup が担当）と重複しないよう、接続詞書き出しは対象外。
    let opener_of = |s: &str| -> String {
        s.chars()
            .take_while(|c| !matches!(c, '、' | '。' | '！' | '？'))
            .take(6)
            .collect()
    };
    let mut op_run: Option<(String, usize, usize)> = None; // (書き出し, 開始行, 本数)
    for (line, s) in &sents {
        let o = opener_of(s);
        // bullet 行は対象外: 「- **Tier 1**…」「- **Tier 2**…」の列は正当な構造で、
        // 接頭が揃うのは当然（2026-07-09 dogfood で実測した FP class）。
        let is_bullet = matches!(s.chars().next(), Some('-' | '*' | '+' | '・' | '#'));
        let eligible = o.chars().count() >= 2 && !connector.is_match(s) && !is_bullet;
        match op_run.as_mut() {
            Some((prev, _, n)) if eligible && *prev == o => *n += 1,
            _ => {
                if let Some((prev, start, n)) = op_run.take()
                    && n >= 3
                {
                    v.push(Violation {
                        line: start,
                        rule: "opener-repetition",
                        severity: Severity::Advisory,
                        msg: format!("同じ書き出し「{prev}…」が {n} 文連続 — 構文を変える"),
                    });
                }
                if eligible {
                    op_run = Some((o, *line, 1));
                }
            }
        }
    }
    if let Some((prev, start, n)) = op_run.take()
        && n >= 3
    {
        v.push(Violation {
            line: start,
            rule: "opener-repetition",
            severity: Severity::Advisory,
            msg: format!("同じ書き出し「{prev}…」が {n} 文連続 — 構文を変える"),
        });
    }

    for (line, s) in &sents {
        let n = s.chars().count();
        if n > max_sentence {
            v.push(Violation {
                line: *line,
                rule: "sentence-length",
                severity: Severity::Hard,
                msg: format!("一文 {n} 字 (> {max_sentence}) — 文を切る（一文一義）"),
            });
        }
        let ten = s.matches('、').count();
        if ten > max_ten {
            v.push(Violation {
                line: *line,
                rule: "max-ten",
                severity: Severity::Hard,
                msg: format!("読点 {ten} 個 (> {max_ten}) — 文を分割するか構造を変える"),
            });
        }
        if let Some(m) = dneg.find(s) {
            v.push(Violation {
                line: *line,
                rule: "double-negative",
                severity: Severity::Hard,
                msg: format!("慣用二重否定「{}…」— 肯定形で言い切る", m.as_str()),
            });
        }
        let hedges: Vec<&str> = hedge.find_iter(s).map(|m| m.as_str()).collect();
        if hedges.len() >= 2 {
            v.push(Violation {
                line: *line,
                rule: "hedge-pileup",
                severity: Severity::Hard,
                msg: format!(
                    "ぼかし連発 ({}) — 言い切るか、根拠を添えて 1 つに絞る",
                    hedges.join("・")
                ),
            });
        }
        if demons.find_iter(s).count() >= 3 {
            v.push(Violation {
                line: *line,
                rule: "demonstrative-chain",
                severity: Severity::Hard,
                msg: "指示語 3 つ以上 — 指す対象を名詞で書き直す".to_string(),
            });
        }
        // 「そのもの」は語彙単位（の×2 を含む）— 連鎖に数えない（2026-07-09 dogfood FP）。
        let masked = title_span.replace_all(s, "◯").replace("そのもの", "◯");
        if let Some(m) = no_chain.find(&masked) {
            // 指示詞（こ/そ/あ/ど）の の は連鎖に数えない: このAのBのC は content 2 で不問。
            let content = m
                .as_str()
                .split('の')
                .filter(|x| !x.is_empty() && !matches!(*x, "こ" | "そ" | "あ" | "ど"))
                .count();
            if content >= 3 {
                v.push(Violation {
                    line: *line,
                    rule: "no-chain",
                    severity: Severity::Hard,
                    msg: format!(
                        "「の」3 連鎖（{}…）— 語順を変えるか複合語/句へ畳む",
                        m.as_str()
                    ),
                });
            }
        }
        if verbose.is_match(s) {
            v.push(Violation {
                line: *line,
                rule: "verbose-potential",
                severity: Severity::Hard,
                msg: "「することができ…」— 「できる」で言い切る（簡潔）".to_string(),
            });
        }
        if let Some(m) = kanji_run.find(s) {
            v.push(Violation {
                line: *line,
                rule: "kanji-run",
                severity: Severity::Advisory,
                msg: format!(
                    "漢字 {} 連続「{}」— 読点や送り仮名で切る（固有名なら無視）",
                    m.as_str().chars().count(),
                    m.as_str()
                ),
            });
        }
        if exclaim.is_match(s) {
            v.push(Violation {
                line: *line,
                rule: "exclamation",
                severity: Severity::Advisory,
                msg: "感嘆符 ！ — 実用文では感情でなく事実で示す（木下）".to_string(),
            });
        }
        if connector.is_match(s) {
            match conn_run.as_mut() {
                Some((_, n)) => *n += 1,
                None => conn_run = Some((*line, 1)),
            }
        } else {
            flush_conn(&mut conn_run, &mut v);
        }
        if polite.is_match(s) {
            polite_lines.push(*line);
        } else if plain.is_match(s) {
            plain_lines.push(*line);
        }
    }
    flush_conn(&mut conn_run, &mut v);

    if !polite_lines.is_empty() && !plain_lines.is_empty() {
        // 少数派の文体を violation として指す（直す対象が明確になる）。
        let (minority, name) = if polite_lines.len() <= plain_lines.len() {
            (&polite_lines, "ですます")
        } else {
            (&plain_lines, "である")
        };
        // 同一行に少数派が複数あっても報告は 1 件（重複ノイズの排除・2026-07-09 実測）。
        let mut seen = std::collections::HashSet::new();
        for l in minority.iter().filter(|l| seen.insert(**l)).take(5) {
            v.push(Violation {
                line: *l,
                rule: "style-mixing",
                severity: Severity::Hard,
                msg: format!(
                    "文体混在: この文だけ{name}体 (敬体 {} 文 / 常体 {} 文) — どちらかへ統一",
                    polite_lines.len(),
                    plain_lines.len()
                ),
            });
        }
    }
    v.sort_by_key(|x| x.line);
    v
}

/// 機械的に安全な自動修正（`check --write`）。現在の対象: `することができ`→`でき`
/// （実行することができます→実行できます）。意味を変え得る修正（二重否定・文体統一・文分割）
/// は対象にしない — 安全に置換できる規則だけを増やす方針。
/// fence 内・inline code 内は「言及」なので触らない（README の規則説明を壊さない）。
/// 返り値は (修正後 text, 修正件数)。
pub fn fix(text: &str) -> (String, usize) {
    let mut out = String::with_capacity(text.len());
    let mut n = 0usize;
    let mut in_fence = false;
    for (i, line) in text.lines().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            out.push_str(line);
            continue;
        }
        if in_fence {
            out.push_str(line);
            continue;
        }
        // backtick で行を分割: 偶数 index = code 外（置換対象）、奇数 = inline code 内。
        for (j, seg) in line.split('`').enumerate() {
            if j > 0 {
                out.push('`');
            }
            if j % 2 == 0 {
                n += seg.matches("することができ").count();
                out.push_str(&seg.replace("することができ", "でき"));
            } else {
                out.push_str(seg);
            }
        }
    }
    if text.ends_with('\n') {
        out.push('\n');
    }
    (out, n)
}

/// CLI entry。Hard violation があれば exit 1（--advisory で 0）。Advisory 規則は報告のみで
/// exit に数えない（MIX tier — judge/人の確認へ回す）。
pub fn run_kinoshita(args: KinoshitaArgs) -> i32 {
    let mut hard = 0usize;
    let mut adv = 0usize;
    let mut report = |label: &str, text: &str| {
        for x in scan(text, args.max_sentence, args.max_ten) {
            let tag = match x.severity {
                Severity::Hard => {
                    hard += 1;
                    ""
                }
                Severity::Advisory => {
                    adv += 1;
                    "·advisory"
                }
            };
            println!("{label}L{}: [{}{tag}] {}", x.line, x.rule, x.msg);
        }
    };
    if args.files.is_empty() {
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf).ok();
        report("", &buf);
    } else {
        for f in &args.files {
            match fs::read_to_string(f) {
                Err(e) => eprintln!("correo kinoshita: {f} 読込失敗: {e} (skip)"),
                Ok(t) => report(&format!("{f}:"), &t),
            }
        }
    }
    if hard == 0 && adv == 0 {
        println!(
            "KINOSHITA PASS: Tier1 違反なし（文長・読点・文体混在・二重否定・ぼかし・指示語・の連鎖・冗長）"
        );
        0
    } else if hard == 0 {
        println!("KINOSHITA PASS: HARD 違反なし（advisory {adv} 件 — judge/人が確認）");
        0
    } else if args.advisory {
        println!("KINOSHITA CANDIDATES: HARD {hard} 件 + advisory {adv} 件（advisory mode）");
        0
    } else {
        println!("KINOSHITA FAIL: {hard} 件（+ advisory {adv} 件）");
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ja(n: usize) -> String {
        "あ".repeat(n)
    }

    #[test]
    fn flags_long_sentence_with_line_number() {
        let text = format!("短い文である。\n{}。", ja(120));
        let v = scan(&text, 100, 4);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].rule, "sentence-length");
        assert_eq!(v[0].line, 2);
    }

    #[test]
    fn flags_too_many_ten() {
        let text = "また、まず、次に、さらに、加えて、最後に述べる。";
        let v = scan(text, 100, 4);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].rule, "max-ten");
    }

    #[test]
    fn flags_idiomatic_double_negative_not_obligation() {
        let bad = "その効果がないことはない。";
        assert_eq!(scan(bad, 100, 4)[0].rule, "double-negative");
        // なければならない は義務表現であって二重否定でない — 誤爆しないこと。
        let ok = "測定は毎回較正しなければならない。";
        assert!(scan(ok, 100, 4).is_empty());
    }

    #[test]
    fn flags_hedge_pileup_but_allows_single_hedge() {
        let bad = "この方式は有効かもしれないと思われる。";
        let v = scan(bad, 100, 4);
        assert!(v.iter().any(|x| x.rule == "hedge-pileup"));
        let ok = "この方式は有効かもしれない。";
        assert!(scan(ok, 100, 4).iter().all(|x| x.rule != "hedge-pileup"));
    }

    #[test]
    fn flags_demonstrative_chain_of_three() {
        let bad = "これがそれを含み、その結果を決める。";
        assert!(
            scan(bad, 100, 4)
                .iter()
                .any(|x| x.rule == "demonstrative-chain")
        );
        let ok = "この方式は測定結果を決める。";
        assert!(
            scan(ok, 100, 4)
                .iter()
                .all(|x| x.rule != "demonstrative-chain")
        );
    }

    #[test]
    fn flags_style_mixing_pointing_at_minority() {
        let text = "本手法は誤差を半減する。\n測定は三回行った。\n結果は表に示します。";
        let v = scan(text, 100, 4);
        let mix: Vec<_> = v.iter().filter(|x| x.rule == "style-mixing").collect();
        assert_eq!(mix.len(), 1, "少数派 1 文だけを指すべき: {:?}", mix.len());
        assert_eq!(mix[0].line, 3);
        // 統一されていれば無違反。
        let uni = "本手法は誤差を半減する。\n測定は三回行った。";
        assert!(scan(uni, 100, 4).is_empty());
    }

    #[test]
    fn flags_no_chain_but_not_demonstratives() {
        let bad = "本手法の評価の結果の妥当性を検証する。";
        assert!(scan(bad, 100, 4).iter().any(|x| x.rule == "no-chain"));
        // この の「の」は指示詞の一部 — content 2 なので不問。
        let ok = "この本の著者の意見を述べる。";
        assert!(scan(ok, 100, 4).iter().all(|x| x.rule != "no-chain"));
        // 『…』引用タイトル内の の は書名の一部 — 連鎖に数えない（dogfood 実測 FP の回帰）。
        let title = "木下是雄『理科系の作文技術』の原則の一部を検査する。";
        assert!(scan(title, 100, 4).iter().all(|x| x.rule != "no-chain"));
    }

    #[test]
    fn flags_verbose_potential() {
        let bad = "同じ結果を再現することができます。";
        assert!(
            scan(bad, 100, 4)
                .iter()
                .any(|x| x.rule == "verbose-potential")
        );
        let ok = "同じ結果を再現できます。";
        assert!(
            scan(ok, 100, 4)
                .iter()
                .all(|x| x.rule != "verbose-potential")
        );
    }

    #[test]
    fn connector_pileup_is_advisory_and_needs_three() {
        let bad = "また、測定を行った。さらに、解析を行った。そして、結論を得た。";
        let v = scan(bad, 100, 4);
        let c: Vec<_> = v.iter().filter(|x| x.rule == "connector-pileup").collect();
        assert_eq!(c.len(), 1);
        assert!(matches!(c[0].severity, Severity::Advisory));
        // 2 連続は不問（普通の文章にもある）。
        let ok = "また、測定を行った。さらに、解析を行った。";
        assert!(
            scan(ok, 100, 4)
                .iter()
                .all(|x| x.rule != "connector-pileup")
        );
    }

    #[test]
    fn flags_predicate_colon_before_block_but_allows_noun_colon() {
        // 英語 "Do the following:" の直訳調（述語＋コロン→箇条書き）。名詞受けは正当。
        let bad = "次の手順で設定を変更します:\n\n- 項目を開く。";
        assert!(
            scan(bad, 100, 4)
                .iter()
                .any(|x| x.rule == "colon-continuation")
        );
        let ok = "使用方法:\n\n- 項目を開く。";
        assert!(
            scan(ok, 100, 4)
                .iter()
                .all(|x| x.rule != "colon-continuation")
        );
    }

    #[test]
    fn flags_long_kanji_run_advisory_but_not_short_terms() {
        // textlint max-kanji-continuous-len 相当。7+ を advisory で。
        let bad = "電子情報通信工学専攻に所属する。";
        let hits: Vec<_> = scan(bad, 100, 4)
            .into_iter()
            .filter(|x| x.rule == "kanji-run")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(matches!(hits[0].severity, Severity::Advisory));
        // 6 字以下の術語（形態素解析器）は誤爆しない。
        assert!(
            scan("形態素解析器を用いる。", 100, 4)
                .iter()
                .all(|x| x.rule != "kanji-run")
        );
    }

    #[test]
    fn flags_exclamation_advisory() {
        assert!(
            scan("これは本当にすごい成果だ！", 100, 4)
                .iter()
                .any(|x| x.rule == "exclamation")
        );
        assert!(
            scan("これは良い成果だ。", 100, 4)
                .iter()
                .all(|x| x.rule != "exclamation")
        );
    }

    #[test]
    fn flags_bullet_template_run_of_three() {
        // 「- **見出し**: 説明」×3 連続 = LLM の定型 layout（革新性：/効率性： の列）。
        let bad = "- **革新性**: 高い。\n- **効率性**: 速い。\n- **拡張性**: 広い。";
        let hits: Vec<_> = scan(bad, 100, 4)
            .into_iter()
            .filter(|x| x.rule == "bullet-template")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(matches!(hits[0].severity, Severity::Advisory));
        let ok = "- **革新性**: 高い。\n- **効率性**: 速い。";
        assert!(scan(ok, 100, 4).iter().all(|x| x.rule != "bullet-template"));
    }

    #[test]
    fn flags_same_opener_three_sentences() {
        let bad = "本製品は、速い。本製品は、安い。本製品は、強い。";
        assert!(
            scan(bad, 100, 4)
                .iter()
                .any(|x| x.rule == "opener-repetition")
        );
        let ok = "本製品は、速い。本製品は、安い。";
        assert!(
            scan(ok, 100, 4)
                .iter()
                .all(|x| x.rule != "opener-repetition")
        );
    }

    #[test]
    fn fix_rewrites_verbose_potential_but_not_mentions() {
        // --write の安全性: 地の文は直す・inline code / fence 内の「言及」は触らない。
        let (out, n) = fix("同じ結果を再現することができます。");
        assert_eq!(out, "同じ結果を再現できます。");
        assert_eq!(n, 1);
        let (out, n) = fix("規則の説明（`することができ`→`でき`）は言及。");
        assert_eq!(n, 0, "inline code 内を書き換えた");
        assert!(out.contains("`することができ`"));
        let (out, n) = fix("```\nすることができます\n```\n本文ですることができます。\n");
        assert_eq!(n, 1, "fence 内を数えた/本文を見逃した");
        assert!(out.contains("```\nすることができます\n```"));
        assert!(out.contains("本文でできます。"));
    }

    #[test]
    fn skips_fence_table_heading_and_keeps_lines() {
        let text = format!(
            "# {}。\n\n```\n{}。\n```\n\n| {}。 |\n\n{}。",
            ja(120),
            ja(120),
            ja(120),
            ja(120)
        );
        let v = scan(&text, 100, 4);
        assert_eq!(v.len(), 1, "構造行/fence が検査対象になった");
        assert_eq!(v[0].line, 9, "fence 除去で行番号がずれた");
    }

    #[test]
    fn joins_sentence_across_hard_wrapped_lines() {
        // 行跨ぎの一文は接続して 1 文として測る（開始行を指す）。
        let text = format!("{}\n{}。", ja(60), ja(60));
        let v = scan(&text, 100, 4);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].rule, "sentence-length");
        assert_eq!(v[0].line, 1);
    }
}
