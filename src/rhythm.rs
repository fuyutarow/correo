// rhythm.rs — 文長・段落構造のリズム単調さ（機械的リズム）の測定 emit。
//
// 出自（2026-07-17 蒸留）: coji/natural-japanese（zenn: natural-japanese-ai-smell-lint・MIT）の
// 7 モデル×406 本の実測。「AI 臭は語彙よりリズムに出る」— 禁止語・翻訳調が皆無でも、文長の
// メリハリの欠如（burstiness）と段落構造の均質さ（3 文段落の量産）がモデル横断で残る指紋だった:
//   - 文長リズム均質の文書発火率: gpt-5.6-sol 88% / Sonnet 5 55% / Fable 5 60%（人間 FP 2.4%）
//   - 段落構造均質: gpt-5.6-sol 52% / Sonnet 5 17% / Fable 5 0%
//   - 逆に語彙系は較正で削除が相次いだ（「最後に」「まさに」は人間の日常語 — 人間側ヒットの 63%）。
//
// 移植でなく蒸留 — 出典の閾値は持ち込まない:
//   (1) 出典は文分割に改行も区切りとして使い（bullet・見出しが「文」に混入）、モーラ近似長
//       （Sudachi reading_form）で測る。correo の sentences() は 。！？ のみで切る別規約 —
//       閾値は分割規約に従属するので、correo 自身の corpus 較正で取り直す（下の較正記録）。
//   (2) モーラでなく字数を使う。Sudachi は optional 依存（coinage feature）で、常時有効の
//       検出器を feature に依存させない。burstiness=(σ−μ)/(σ+μ) は CV の単調変換でスケール
//       不変 — 字/モーラ比の文間変動（漢字密度差）は二次項で、較正を字数側で取れば成立する。
//   (3) 地の文だけを測る: 終端記号（。！？）で閉じた文のみ・bullet 行と blockquote は除外
//       （bullet の列は並列構造で長さが揃うのが正当、引用は他人の文体 — rhetoric guard #3 と同じ）。
//
// 較正（2026-07-17・correo 自身の文分割規約での corpus 実測。人間 16 本 = 青空随筆 8〔寺田寅彦 4・
// 中谷宇吉郎ほか 4〕・pre-2020 web 技術記事 5・官公庁 3 / AI 30 本 = sonnet 生成の実用文・文体
// 指示なし〔短め 20 + 地の文主体の長め 10〕）:
//   flat-rhythm    burstiness 分布: AI -0.32〜-0.51 / 青空 -0.26〜-0.34 / web -0.19〜-0.36 /
//                  官公庁 +0.20〜-0.36。閾値 -0.40 で人間 FP 0/15（n≥30 の適格文書）・
//                  AI 検出 8/11（73%・行単位 bullet 除外の是正後の実測）。最近接の人間は官公庁
//                  ガイドライン gov-3 の -0.362（margin 0.038）— 法令・手順の文体は文長が
//                  揃いがちで、これ以上の欲張りは人間の実務文を撃つ。境界域（-0.36〜-0.40）は
//                  沈黙 = judge の領分。
//   uniform-paragraphs 文数 cv: 閾値 0.26 で人間 FP 0/8（適格文書）・AI 検出 7/26（27%・
//                  出典実測の Sonnet 17% と同 tier）。最近接の人間は青空随筆の 0.310
//                  （margin 0.05）。段落平均文数の [2,6] ゲートが人間の両翼を守る（下の (c)(d)）。
//   ※出典の閾値（burstiness -0.24 等）は移植していない — 分割規約・長さ単位が違う（冒頭 (1)(2)）。
//
// 反証 guard（rhetoric と同じ設計思想を継承）:
//   (a) 全て advisory・測定値を emit し裁定は judge へ — リズムは文体であって欠陥の確定でない。
//   (b) 最低分母 30 文 / 6 段落 — 短文書の統計の暴発を封じる（rhetoric・density と同じ規約）。
//       短い実務文書（週報・議事録級）は計測不能として沈黙する — これは仕様（沈黙＞誤爆）。
//   (c) uniform-paragraphs は段落平均 2 文未満で沈黙 — 1 文 1 段落の人間的 web 文体（改行多用）は
//       CV=0 に退化するが、それは段落構成の選好であってリズムの指紋でない（較正で web-1/3/4 が
//       このゲートに守られた実測）。
//   (d) 段落平均 6 文超も沈黙 — 長い均質段落は古典随筆の正当な文体（較正で青空 2 本が
//       pmu 10〜13.7 に座った実測）。量産される定型段落は短い（3 文段落の量産）が指紋の本体。
//
// 見送り（出典にあるが採らない・裁定の記録）:
//   - 体言止めゼロ（出典: essay で人間 60% vs AI 0%）— correo の対象は実用文で、体言止めの欠如は
//     実用文では正当（木下の規範文書がそもそも使わない）。essay 向けの知見は輸入しない。
//   - lag-1 自己相関（出典でも info 止まり・sweep で弁別力の実測なし）— 指標を増やさない。
use crate::report::{Severity, Violation};
use regex::Regex;

/// 率の最低分母（文数）。これ未満の文書は計測不能として何も emit しない（rhetoric と同じ規約）。
const MIN_SENTENCES: usize = 30;
/// flat-rhythm の burstiness 閾値（較正記録参照: 最近接の人間 gov-3 -0.362 の下に margin 0.038）。
const BURSTINESS_MAX: f64 = -0.40;
/// uniform-paragraphs の最低分母（非 bullet 段落数）。
const MIN_PARAGRAPHS: usize = 6;
/// uniform-paragraphs の段落平均文数の床（反証 guard (c): 1 文 1 段落文体は対象外）。
const PARA_MEAN_MIN: f64 = 2.0;
/// uniform-paragraphs の段落平均文数の上限（反証 guard (d): 長い均質段落の古典文体は対象外）。
const PARA_MEAN_MAX: f64 = 6.0;
/// uniform-paragraphs の文数 CV 閾値（較正記録参照: 最近接の人間 0.310 の下に margin 0.05）。
const PARA_CV_MAX: f64 = 0.26;

/// 文書単位のリズム統計（sweep・judge 連携用に公開。閾値判定は scan が持つ）。
pub struct RhythmStats {
    /// 計測対象になった地の文の文数（終端記号で閉じ・bullet/blockquote 除外後）。
    pub sentences: usize,
    pub mean_chars: f64,
    pub stdev_chars: f64,
    /// (σ−μ)/(σ+μ)。負に大きいほど文長が均一（機械的リズム）。
    pub burstiness: f64,
    /// 段落構造（非 bullet・1 文以上の散文段落）の統計。
    pub paragraphs: usize,
    pub para_mean_sentences: f64,
    pub para_cv: f64,
    /// finding が指す行（最初の計測対象文・最初の対象段落）。
    first_sentence_line: usize,
    first_paragraph_line: usize,
}

fn mean(xs: &[f64]) -> f64 {
    xs.iter().sum::<f64>() / xs.len() as f64
}

fn pstdev(xs: &[f64], mu: f64) -> f64 {
    (xs.iter().map(|x| (x - mu).powi(2)).sum::<f64>() / xs.len() as f64).sqrt()
}

/// 文書のリズム統計を計測する。閾値判定はしない（sweep が分布を直接見るための口）。
pub fn doc_stats(text: &str) -> RhythmStats {
    // 地の文の文: 終端記号で閉じている・bullet 行/blockquote 行に発しない。
    // 除外は**行単位**で行う — sentences() は 。！？ ごとに分割するため、複数文の bullet 項
    // （「- 一文目。二文目。」）の 2 文目以降は行頭記号を失い、文頭の接頭判定では漏れる
    // （2026-07-17 code-review CONFIRMED の是正: 全行が 2 文 bullet の文書で地の文 0 のはずが
    // 半数が標本に混入していた。bullet 列は長さが揃うため burstiness を機械側へ汚染する）。
    // 文の開始行が bullet/引用行なら、その行に発する全ての文を標本から外す。
    // 段落末尾の未終端 fragment（bullet 項・見出し残骸）は終端記号条件が落とす。
    let marker = Regex::new(r"^(?:>|[-*+][ \t]|・|[0-9０-９]+[.．)])").unwrap();
    let marker_lines: std::collections::HashSet<usize> = text
        .lines()
        .enumerate()
        .filter(|(_, l)| marker.is_match(l.trim_start()))
        .map(|(i, _)| i + 1)
        .collect();
    let sents: Vec<(usize, String)> = crate::prose::sentences(text)
        .into_iter()
        .filter(|(line, s)| !marker_lines.contains(line) && s.ends_with(['。', '！', '？']))
        .collect();
    let lengths: Vec<f64> = sents
        .iter()
        .map(|(_, s)| s.chars().count() as f64)
        .collect();
    let (mu, sd, b) = if lengths.is_empty() {
        (0.0, 0.0, 0.0)
    } else {
        let mu = mean(&lengths);
        let sd = pstdev(&lengths, mu);
        let b = if sd + mu > 0.0 {
            (sd - mu) / (sd + mu)
        } else {
            0.0
        };
        (mu, sd, b)
    };

    // 段落構造: 非 bullet・非引用の散文段落ごとの文数（終端記号の個数）。0 文の段落
    // （見出し残骸等）は標本にしない。
    let paras: Vec<(usize, usize)> = crate::prose::prose_units(text)
        .into_iter()
        .filter(|u| !u.is_bullet && !u.text.trim_start().starts_with('>'))
        .map(|u| {
            let n = u
                .text
                .chars()
                .filter(|c| matches!(c, '。' | '！' | '？'))
                .count();
            (u.line, n)
        })
        .filter(|(_, n)| *n >= 1)
        .collect();
    let counts: Vec<f64> = paras.iter().map(|(_, n)| *n as f64).collect();
    let (pmu, pcv) = if counts.is_empty() {
        (0.0, 0.0)
    } else {
        let pmu = mean(&counts);
        let pcv = if pmu > 0.0 {
            pstdev(&counts, pmu) / pmu
        } else {
            0.0
        };
        (pmu, pcv)
    };

    RhythmStats {
        sentences: lengths.len(),
        mean_chars: mu,
        stdev_chars: sd,
        burstiness: b,
        paragraphs: counts.len(),
        para_mean_sentences: pmu,
        para_cv: pcv,
        first_sentence_line: sents.first().map(|(l, _)| *l).unwrap_or(1),
        first_paragraph_line: paras.first().map(|(l, _)| *l).unwrap_or(1),
    }
}

/// 文書単位のリズム検査。全て advisory — リズムは文体の指紋であって欠陥の確定でない（judge へ）。
pub fn scan(text: &str) -> Vec<Violation> {
    let st = doc_stats(text);
    let mut v = Vec::new();

    if st.sentences >= MIN_SENTENCES && st.burstiness < BURSTINESS_MAX {
        v.push(Violation {
            line: st.first_sentence_line,
            rule: "flat-rhythm",
            severity: Severity::Advisory,
            msg: format!(
                "flat sentence-length rhythm: burstiness={:.2} (n={}, mean {:.0} chars, sd {:.0}; calibrated floor {BURSTINESS_MAX}) — machine-even sentence lengths; mix short and long sentences. Route to judge",
                st.burstiness, st.sentences, st.mean_chars, st.stdev_chars
            ),
        });
    }

    if st.paragraphs >= MIN_PARAGRAPHS
        && st.para_mean_sentences >= PARA_MEAN_MIN
        && st.para_mean_sentences <= PARA_MEAN_MAX
        && st.para_cv < PARA_CV_MAX
    {
        v.push(Violation {
            line: st.first_paragraph_line,
            rule: "uniform-paragraphs",
            severity: Severity::Advisory,
            msg: format!(
                "uniform paragraph structure: {} paragraphs of ~{:.1} sentences each (cv={:.2} < {PARA_CV_MAX}) — templated paragraphing (e.g. mass-produced 3-sentence paragraphs); merge or split by content. Route to judge",
                st.paragraphs, st.para_mean_sentences, st.para_cv
            ),
        });
    }

    v.sort_by_key(|x| x.line);
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ほぼ等長の文を n 本（機械的リズム: burstiness ≈ CV≈0 → -1 に近い）。
    fn monotone(n: usize) -> String {
        (0..n)
            .map(|i| {
                format!(
                    "第{:02}回の測定では基準信号との相互相関から時間遅れを推定して補正した。",
                    i
                )
            })
            .collect::<Vec<_>>()
            .join("")
    }

    /// 短長を強く混ぜた n 対（人間的リズム: CV が大きい）。
    fn varied(n: usize) -> String {
        (0..n)
            .map(|i| {
                format!(
                    "違う。第{i}回の測定では基準信号との相互相関から時間遅れをまず推定し、続いて振幅の非線形性を多項式回帰で補正し、最後に残差が白色雑音の水準へ収まることを確認するところまでを一続きの手順として実施した。"
                )
            })
            .collect::<Vec<_>>()
            .join("")
    }

    #[test]
    fn flat_rhythm_fires_on_monotone_prose_but_not_on_varied() {
        let hits = scan(&monotone(35));
        assert!(
            hits.iter().any(|x| x.rule == "flat-rhythm"),
            "等長 35 文で flat-rhythm が発火しない"
        );
        assert!(
            hits.iter()
                .all(|x| matches!(x.severity, Severity::Advisory))
        );
        assert!(
            scan(&varied(20)).iter().all(|x| x.rule != "flat-rhythm"),
            "短長混在の文書で誤爆"
        );
    }

    #[test]
    fn short_documents_are_not_measured() {
        // 最低分母 30 文 — どれだけ単調でも短文書は計測不能として沈黙する。
        assert!(scan(&monotone(29)).is_empty());
    }

    #[test]
    fn bullets_and_quotes_are_excluded_from_the_sample() {
        // bullet の列は並列構造で長さが揃うのが正当 — 標本に入れない。
        // 29 本の等長 bullet ＋ 地の文 10 文では、地の文が 30 文に満たず沈黙する。
        let bullets: String = (0..29)
            .map(|i| format!("- 項目{:02}は前回と同じ手順で確認を実施した。\n", i))
            .collect();
        let doc = format!("{bullets}\n{}", monotone(10));
        assert!(
            scan(&doc).iter().all(|x| x.rule != "flat-rhythm"),
            "bullet 行が文長標本に混入した"
        );
        // blockquote も同様（引用は他人の文体）。
        let quoted: String = (0..35)
            .map(|i| {
                format!(
                    "> 第{:02}回の測定では基準信号との相互相関から時間遅れを推定して補正した。\n",
                    i
                )
            })
            .collect();
        assert!(
            scan(&quoted).iter().all(|x| x.rule != "flat-rhythm"),
            "blockquote が文長標本に混入した"
        );
        // 回帰（2026-07-17 code-review CONFIRMED）: 複数文の bullet 項の 2 文目以降。
        // sentences() は 。ごとに分割するため 2 文目は行頭記号を失う — 行単位の除外で
        // 全文が標本から外れ、地の文 0 文の文書は沈黙しなければならない。
        let multi_sentence_bullets: String = (0..40)
            .map(|i| format!("- 項目{:02}は最初の文。ここに続く二文目も等長で並ぶ。\n", i))
            .collect();
        assert_eq!(
            doc_stats(&multi_sentence_bullets).sentences,
            0,
            "複数文 bullet の 2 文目以降が地の文標本に漏れた"
        );
        assert!(scan(&multi_sentence_bullets).is_empty());
    }

    #[test]
    fn uniform_paragraphs_fires_on_templated_three_sentence_paragraphs() {
        // 3 文段落 ×8 の量産 = 定型段落。段落内の文長は変えて flat-rhythm と独立に検査する。
        let doc: String = (0..8)
            .map(|i| {
                format!(
                    "測定を行った。第{i}回は装置の較正から着手し、基準信号との相互相関で時間遅れを推定する工程を先に済ませた。結果は表に整理した。\n\n"
                )
            })
            .collect();
        let hits = scan(&doc);
        assert!(
            hits.iter().any(|x| x.rule == "uniform-paragraphs"),
            "3 文段落の量産で uniform-paragraphs が発火しない: {:?}",
            hits.iter().map(|x| x.rule).collect::<Vec<_>>()
        );
    }

    #[test]
    fn varied_paragraphs_and_one_sentence_style_do_not_fire() {
        // 文数 1〜5 のばらついた段落構成 — 発火しない。
        let mut doc = String::new();
        for n in [1usize, 4, 2, 5, 1, 3, 2, 5] {
            for i in 0..n {
                doc.push_str(&format!("段落内の第{i}文として測定と補正の手順を記した。"));
            }
            doc.push_str("\n\n");
        }
        assert!(
            scan(&doc).iter().all(|x| x.rule != "uniform-paragraphs"),
            "ばらついた段落構成で誤爆"
        );
        // 1 文 1 段落の web 文体は CV=0 に退化するが対象外（反証 guard (c)）。
        let one_liner: String = (0..10)
            .map(|i| format!("第{i}段落はこの一文だけで改行する文体で書いた。\n\n"))
            .collect();
        assert!(
            scan(&one_liner)
                .iter()
                .all(|x| x.rule != "uniform-paragraphs"),
            "1 文 1 段落文体で誤爆（PARA_MEAN_MIN ガードが効いていない）"
        );
        // 長い均質段落（古典随筆の文体・較正の青空実測 pmu 10〜13.7）も対象外（反証 guard (d)）。
        let classic: String = (0..7)
            .map(|_| {
                let para: String = (0..13)
                    .map(|i| format!("其の第{i}の事情に就いては別に記す所があつた。"))
                    .collect();
                format!("{para}\n\n")
            })
            .collect();
        assert!(
            scan(&classic)
                .iter()
                .all(|x| x.rule != "uniform-paragraphs"),
            "長い均質段落の古典文体で誤爆（PARA_MEAN_MAX ガードが効いていない）"
        );
    }
}
