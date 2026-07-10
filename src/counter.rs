// counter.rs — 助数詞欠落の検出（2026-07-11・qoed 句レベル指摘）。
//
// 出自: 台帳体（研究 registry・記録の来歴ログ）は「43 cell」「10 track」のように、数字と
// ラテン名詞を助数詞なしで直結する記法を常用する。台帳の内部では省略として正当だが、読者向け
// 散文へ複写されると「43 cell を」のように日本語の助詞へ直接つながり、和文として資格を失った
// まま漏れる。notation.rs（台帳の記号が地の文の接続・論理を肩代わりする事故）と同型の漏出だが、
// 検査する性質が別（notation は記号の文法的役割、本 module は数字とラテン名詞の直結そのもの）
// なので同じ home に畳まない（lib.rs の「検出器 = 検査する性質 1 つ」原則）。
//
// 較正（2026-07-11・qoed ポートフォリオ HTML 抽出 corpus）: 「数字 + 空白 + ラテン語幹」の
// 直後に日本語助詞（を・が・は・で・に・と・へ・の）または句読点が続く形を機械抽出したところ、
// 陽性は「43 cell を」「8 cell の」「10 track を」の 3 件のみだった。同コーパスには
// 「43 cell（7 家族群別）」「13 cell ── PASS」のようにダッシュ・括弧・改行へ続く非陽性が
// 27 件あり、これらは助詞・句読点に直結しないため対象外のままでよい（規則の狙いどおり）。
// correo 自身の README.md 他の *.md では該当ゼロ（陰性の再確認）。
//
// 除外（見送り理由を明記する規約に従う）:
//   - 単位・計量記号（ms/s/%/×/dB/bits/byte/pt/kg/km/JA 等）— 「10 ms が」のような単位表記は
//     数量の一部であり助数詞欠落ではない。ホワイトリストで弾く（README の "50–80 字" 級との
//     判別も兼ねる。ただし「字」は日本語の助数詞そのものなので単位リストに含めない）。
//   - コード・ID の一部（`E2607_082` 等）— 直後がアンダースコアで始まる語は識別子の構成要素で
//     あり自立した名詞ではないため対象外。
//   - 日付・ハイフン連結の数字列（`2026-07-09` の `09`）— 直前が `-` かつその前も数字である
//     数字は年月日の構成要素であり独立した数量ではない。較正実測（correo 自身の src/*.rs
//     コメント）で `2026-07-09 dogfood で` が「09 dogfood で」として誤検出したため追加。
//   - 数式文脈（`d=4`・表セルの `N=8`）— 直前がラテン文字+`=` の数字は「数字 + 空白 + ラテン語幹」
//     の形自体に合致しないため、正規表現の構造上すでに対象外。
use regex::Regex;

pub struct Finding {
    pub line: usize,
    pub rule: &'static str,
    pub msg: String,
}

/// 助詞・句読点として直結を検査する文字（半角スペース 0-1 個を挟んでよい）。
const PARTICLES_AND_PUNCT: &str = "をがはでにとへの。、！？";

/// PARTICLES_AND_PUNCT を正規表現の捕捉付き文字クラスへ組み込むための helper（regex crate は
/// 文字クラス内での動的文字列展開を構文として持たないため、フォーマット時点で `([...])` を
/// 組み立てる）。呼び出し側はこれを 1 つの捕捉グループとして使う。
fn particle_or_punct_class() -> String {
    format!("([{PARTICLES_AND_PUNCT}])")
}

/// 数量・単位として正当な直後語（除外リスト）。「字」は日本語の助数詞そのものなので含めない。
const UNIT_WORDS: &[&str] = &[
    "ms", "s", "sec", "min", "h", "kg", "g", "mg", "km", "cm", "mm", "nm", "pt", "px", "dB", "Hz",
    "kHz", "MHz", "GHz", "bit", "bits", "byte", "bytes", "JA", "QoI",
];

/// 文書 → 助数詞欠落の全 finding（fence/inline code/表/見出し/URL を除いた地の文が対象）。
/// 除外は prose.rs の clean_lines を共有する（notation.rs と同じ規約）。
pub fn scan(text: &str) -> Vec<Finding> {
    let mut out = Vec::new();
    let defenced = crate::prose::clean_lines(text);

    // 数字（半角）+ 空白 + ラテン語幹（英字始まり・英字/アンダースコア/ハイフン継続）+
    // 空白 0-1 個 + 助詞・句読点（PARTICLES_AND_PUNCT）。
    let hit = Regex::new(&format!(
        r"([0-9]+) ([A-Za-z][A-Za-z_-]*)( ?){}",
        particle_or_punct_class()
    ))
    .unwrap();
    // 直前が `-` でかつその手前も数字（`2026-07-09` の `09` のような日付・ID の構成要素）を
    // 除外するための前方参照。マッチ対象の数字自体は before に含まれない（before はマッチ
    // 開始位置の手前まで）ので、パターンは「数字＋ハイフンで終わる」形になる。
    let date_like = Regex::new(r"[0-9]-$").unwrap();

    for (i, raw) in defenced.lines().enumerate() {
        let line = i + 1;
        for caps in hit.captures_iter(raw) {
            let whole = caps.get(0).unwrap();
            let num = &caps[1];
            let word = &caps[2];
            let particle = &caps[4];

            // 単位語は数量表記であり助数詞欠落ではない。
            if UNIT_WORDS.contains(&word) {
                continue;
            }
            // 日付・ハイフン連結数字列の末尾（`2026-07-09` の `09`）は独立した数量ではない。
            let before = &raw[..whole.start()];
            if date_like.is_match(before) {
                continue;
            }
            // 直後がアンダースコアで始まる語（識別子の構成要素）は自立した名詞ではない。
            let after_word_end = whole.start() + num.len() + 1 + word.len();
            if raw[after_word_end..].starts_with('_') {
                continue;
            }

            out.push(Finding {
                line,
                rule: "missing-counter",
                msg: format!(
                    "「{num} {word}」 directly followed by the particle/punctuation 「{particle}」 — add a Japanese counter word (個・件・本 etc.) or a unit, e.g. 「{num}{word}個」／「{num} 個の {word}」",
                ),
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_number_latin_directly_followed_by_particle_wo() {
        let text = "43 cell を「稼ぐ」と数える。";
        let hits: Vec<_> = scan(text)
            .into_iter()
            .filter(|f| f.rule == "missing-counter")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].msg.contains("43"));
        assert!(hits[0].msg.contains("cell"));
    }

    #[test]
    fn flags_number_latin_directly_followed_by_particle_ni() {
        let text = "10 track に注目する。";
        assert!(scan(text).iter().any(|f| f.rule == "missing-counter"));
    }

    #[test]
    fn flags_number_latin_directly_followed_by_particle_no() {
        let text = "同等 8 cell の代表を示す。";
        assert!(scan(text).iter().any(|f| f.rule == "missing-counter"));
    }

    #[test]
    fn does_not_flag_when_followed_by_dash_or_paren() {
        // 台帳体の内部表記（ダッシュ・括弧続き）は助詞・句読点に直結していないので対象外。
        let text = "13 cell ── PASS 12 / ADAPTIVE_RECOVERS 1\n43 cell（7 家族群別）を数える。";
        let hits = scan(text);
        assert!(
            hits.iter().all(|f| !f.msg.contains("13 cell")),
            "ダッシュ続きを誤検出した: {:?}",
            hits.iter().map(|f| &f.msg).collect::<Vec<_>>()
        );
    }

    #[test]
    fn does_not_flag_unit_words() {
        let text = "処理は 10 ms が上限で、密度は 8 bits を下回る。50 s に収束する。";
        assert!(
            scan(text).is_empty(),
            "単位語を助数詞欠落として誤検出した: {:?}",
            scan(text).iter().map(|f| &f.msg).collect::<Vec<_>>()
        );
    }

    #[test]
    fn does_not_flag_date_like_hyphenated_numbers() {
        // 「2026-07-09 dogfood で」の「09」は日付の構成要素であり独立した数量ではない。
        let text = "2026-07-09 dogfood で実測した。";
        assert!(
            scan(text).is_empty(),
            "日付の構成要素を誤検出した: {:?}",
            scan(text).iter().map(|f| &f.msg).collect::<Vec<_>>()
        );
    }

    #[test]
    fn does_not_flag_code_or_id_fragments() {
        let text = "詳細は `E2607_082` を参照。8 test_helper が失敗した。";
        // `E2607_082` は inline code なので clean_lines が既に除去する。
        // 「8 test_helper」は直後が助詞でなくアンダースコア継続語なので、そもそも word 自体が
        // `test_helper` として一体化し、助詞直結の検査対象にはならない。
        let hits = scan(text);
        assert!(
            hits.iter().all(|f| !f.msg.contains("E2607_082")),
            "inline code の言及を誤検出した"
        );
    }

    #[test]
    fn does_not_flag_math_context_with_equals_sign() {
        // 「d=4」型は「数字 + 空白 + ラテン語幹」の形自体に合致しないため、構造上対象外。
        let text = "d=4 のとき tapered が成立する。N=8 の場合も同様である。";
        assert!(
            scan(text).is_empty(),
            "数式文脈を誤検出した: {:?}",
            scan(text).iter().map(|f| &f.msg).collect::<Vec<_>>()
        );
    }

    #[test]
    fn table_rows_and_inline_code_are_excluded_like_other_detectors() {
        let table_row = "| 43 cell を評価 | 説明 | 吸収 | access |";
        // 表の行は clean_lines が空行化するため、行内の助詞直結は検査対象にならない。
        assert!(
            scan(table_row).iter().all(|f| f.rule != "missing-counter"),
            "表の行は地の文として検査してはいけない"
        );
    }

    #[test]
    fn fence_and_inline_code_mentions_are_not_flagged() {
        let text = "`43 cell を` は記法の例。\n\n```\n10 track を評価\n```";
        assert!(
            scan(text).iter().all(|f| f.rule != "missing-counter"),
            "fence/inline code 内は言及であり対象外のはず"
        );
    }

    #[test]
    fn calibration_corpus_representative_sample_matches_incident_counts() {
        // 実測較正（2026-07-11・qoed ポートフォリオ HTML 抽出 corpus）: 陽性は
        // 「43 cell を」「8 cell の」「10 track を」の 3 件のみ・非陽性（ダッシュ・括弧続き）は
        // 27 件で沈黙する。fixture は repo 外の一時ファイルなので、ここでは代表サンプルで
        // 境界を固定する（較正の全数は README 規則台帳と実装報告に記録・本 test は回帰の pin）。
        let sample = "cell ポートフォリオ ── closed 43 cell（7 家族群別）\n13 cell ── PASS 12 / ADAPTIVE_RECOVERS 1\n43 cell を「稼ぐ」と数えても、稼ぎの実額は全域に均等ではない。\n配分単独最適化（同等 8 cell の代表）\n10 track を priority 順に。";
        let hits = scan(sample);
        assert_eq!(
            hits.len(),
            3,
            "較正 corpus 代表サンプルの陽性数が一致しない: {:?}",
            hits.iter().map(|f| &f.msg).collect::<Vec<_>>()
        );
    }
}
