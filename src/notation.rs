// notation.rs — 台帳記法の読者面への漏出検出（2026-07-11・台帳体プロジェクトの実務事例）。
//
// 出自: 台帳体（研究 registry・記録の来歴ログ）は矢印・全角イコール・中点連結を圧縮記法として
// 常用する。これは台帳の内部では正当（実測: 台帳 file 1 本で矢印数十件・全角イコール数十件、
// prose lint 通過済み記録でも矢印・全角イコールは少数残る）。しかし台帳体の文書から読者向け
// 散文（配布 HTML 等）へ複写されるとき、記号が文の接続・論理・列挙の仕事をしたまま漏れる事故が
// 実測された ── 台帳体の実務文書から読者向け文書へ複写された HTML 1 枚で、矢印×18・
// 全角イコール×10・括弧内 3 連結×4 を実測した（2026-07）。
//
// 検出器 = 検査する性質 1 つ（lib.rs の原則）: 本 module は「台帳記法が地の文の接続・論理の
// 役割を肩代わりしている」ことだけを見る。rhetoric.rs の「——」密度（文体の指紋・composite・
// 文書単位の rate）とは別の性質 ── notation は個々の記号が担う文法的役割（接続詞・断定・列挙）
// を機械的に肩代わりしているかを見る、per-instance の advisory。rhetoric との重複防止として、
// 「——」自体はここで扱わない（rhetoric.rs が所有）。
//
// 規則（全て advisory・行番号つき per-instance）:
//   arrow          矢印（→・⇒・⟹）が地の文に出現。fence/inline code は言及なので対象外。
//   fullwidth-eq   連結用の全角イコール＝。左右どちらかが日本語文字（数式的な使用＝両側とも
//                  変数・数値のみの「x＝1」型と区別する。較正実測は下記 fw_eq 定義部）。
//   paren-nakaten  括弧内で「・」により 3 要素以上を連結（（）と () の両方）。
//
// 見送り: 波ダッシュ/チルダ「約」用法（〜12ms 等）は数値表記の一部として技術文書に高頻度で
// 正当出現し（README 自身の "50–80 字" 級の範囲表記や単位表記と判別が薄い）、"約" の意図か
// 範囲（12〜20）の意図かを記号だけから機械的に分けられない。見送り理由をここに記録する
// （spec が明示的に許可した見送り）。
use regex::Regex;

pub struct Finding {
    pub line: usize,
    pub rule: &'static str,
    pub msg: String,
}

/// 文書 → 台帳記法漏出の全 finding（fence/inline code/表/見出し/URL を除いた地の文が対象）。
/// 除外は prose.rs の clean_lines を共有する（表 `|` 行に潜む「・」3連結や inline code 内の
/// 矢印を誤って地の文として拾わないため・2026-07-11 是正: 独自の strip_fences のみでは表の
/// セル内容が地の文として素通りしていた）。
pub fn scan(text: &str) -> Vec<Finding> {
    let mut out = Vec::new();
    let defenced = crate::prose::clean_lines(text);

    let arrow = Regex::new(r"→|⇒|⟹").unwrap();
    // 全角イコール＝の直前 or 直後が日本語文字（ひらがな/カタカナ/漢字）＝連結用法。
    // 較正実測（2026-07-11・台帳体 registry file 86 件抽出）: 純粋な数式的使用（変数・数値が
    // 両側とも欧文/数字のみ: 「x＝1」型）はほぼ皆無で、大半は日本語節と英数字識別子を跨ぐ
    // 台帳連結（「採択条件＝workflow」「次＝purity」等）— 両側 JA 限定だと 86 件中 27 件しか
    // 拾えず（実測）、識別子跨ぎの過半を見逃す。「片側だけ JA」まで広げて連結用法を捕まえつつ、
    // 両側とも非 JA（真に数式的: 「x＝1」）は対象外にして区別を残す。
    let fw_eq =
        Regex::new(r"[\p{Hiragana}\p{Katakana}\p{Han}]＝|＝[\p{Hiragana}\p{Katakana}\p{Han}]")
            .unwrap();
    // 括弧内の「・」3 要素以上連結。全角/半角括弧の両方・入れ子の丸括弧は対象外（単純な非貪欲）。
    let paren_zenkaku = Regex::new(r"（([^（）]*)）").unwrap();
    let paren_hankaku = Regex::new(r"\(([^()]*)\)").unwrap();

    for (i, raw) in defenced.lines().enumerate() {
        let line = i + 1;
        let clean = raw;

        for m in arrow.find_iter(clean) {
            out.push(Finding {
                line,
                rule: "arrow",
                msg: format!(
                    "ledger arrow 「{}」 in reader-facing prose — write the transition in words (…から…へ／…の結果) or keep the notation only inside a table/code",
                    m.as_str()
                ),
            });
        }

        for m in fw_eq.find_iter(clean) {
            // 表示は match（1-2字）でなく周辺文脈を出す — 「み＝上」のような断片は読めない。
            let ctx = context(clean, m.start(), m.end());
            out.push(Finding {
                line,
                rule: "fullwidth-eq",
                msg: format!(
                    "ledger 「＝」 connector 「{ctx}」 — write 「とは」／「は…である」 instead of the ledger equals-sign shorthand",
                ),
            });
        }

        for caps in paren_zenkaku
            .captures_iter(clean)
            .chain(paren_hankaku.captures_iter(clean))
        {
            let inner = &caps[1];
            // clean_lines の inline code 除去（`E2607_078` 等）は中身を空文字にするため、
            // コード片の言及を「・」で列挙した括弧が「・・」の空要素だけ残ることがある
            // （2026-07-11 実測: R2607_026 の `cell(...)` 型）。空要素は実要素でないので、
            // 非空 segment の個数で 3 要素以上を判定する（言及の列挙を実要素として誤検出しない）。
            let items: Vec<&str> = inner.split('・').filter(|s| !s.trim().is_empty()).collect();
            if items.len() >= 3 {
                out.push(Finding {
                    line,
                    rule: "paren-nakaten",
                    msg: format!(
                        "parenthetical 「・」-joined list of {}+ items 「{}」 — write it as prose (A、B、C の3点) or an actual bullet list, not a packed aside",
                        items.len(),
                        inner
                    ),
                });
            }
        }
    }
    out
}

/// byte 範囲 [start,end) の周辺 ±8 字（文字境界で安全に切る）を抜粋する ── fullwidth-eq の
/// match 自体は 1-2 字しかないため、そのままでは読めない断片になる（メッセージの可読性）。
fn context(s: &str, start: usize, end: usize) -> String {
    let before = &s[..start];
    let after = &s[end..];
    let before_tail: String = before
        .chars()
        .rev()
        .take(8)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    let after_head: String = after.chars().take(8).collect();
    format!("{before_tail}{}{after_head}", &s[start..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_arrows_with_line_numbers() {
        let text = "第一段階から第二段階へ。\n手順A → 手順B → 手順C の順で進む。";
        let hits: Vec<_> = scan(text)
            .into_iter()
            .filter(|f| f.rule == "arrow")
            .collect();
        assert_eq!(hits.len(), 2, "→ が2箇所ある行から2件出るべき");
        assert!(hits.iter().all(|f| f.line == 2));
        assert!(scan("矢印なしの文。").iter().all(|f| f.rule != "arrow"));
    }

    #[test]
    fn flags_double_and_triple_arrow_variants() {
        assert!(
            scan("状態Aから状態Bへ⇒最終状態Cに至る。")
                .iter()
                .any(|f| f.rule == "arrow")
        );
        assert!(
            scan("検査順序⟹凍結予測の決着。")
                .iter()
                .any(|f| f.rule == "arrow")
        );
    }

    #[test]
    fn arrows_in_fence_or_inline_code_are_mentions_not_flagged() {
        assert!(
            scan("`a → b` は記法の例。\n\n```\nA → B\n```")
                .iter()
                .all(|f| f.rule != "arrow"),
            "fence/inline code 内は言及であり対象外のはず"
        );
    }

    #[test]
    fn flags_fullwidth_equals_between_japanese_chars_but_not_math_usage() {
        let bad = "設計対象＝下流に渡る誘導実験である。";
        assert!(scan(bad).iter().any(|f| f.rule == "fullwidth-eq"));
        // 数式的な使用（両側とも非日本語文字）は連結用法と区別して対象外。
        let math = "x＝1 のとき成立する。";
        assert!(
            scan(math).iter().all(|f| f.rule != "fullwidth-eq"),
            "数式的な x＝1 を連結用法として誤検出した"
        );
    }

    #[test]
    fn flags_fullwidth_equals_when_only_one_side_is_japanese() {
        // 較正実測（台帳体 registry file）: 大半は日本語節と英数字識別子を跨ぐ台帳連結
        // （「採択条件＝workflow」「次＝purity」型）── 両側 JA 限定だと過半を見逃す。
        assert!(
            scan("採択条件＝workflow 組込みを要する。")
                .iter()
                .any(|f| f.rule == "fullwidth-eq"),
            "日本語→識別子の片側連結を見逃した"
        );
        assert!(
            scan("残＝purity gated の状態を指す。")
                .iter()
                .any(|f| f.rule == "fullwidth-eq"),
            "識別子→日本語の片側連結を見逃した"
        );
    }

    #[test]
    fn flags_three_or_more_nakaten_joined_items_in_parens_both_bracket_styles() {
        let zenkaku = "介入 campaign（配分・積基底・interface）の並置を検査する。";
        assert!(scan(zenkaku).iter().any(|f| f.rule == "paren-nakaten"));
        let hankaku = "評価軸(精度・速度・再現性)を並べる。";
        assert!(scan(hankaku).iter().any(|f| f.rule == "paren-nakaten"));
        // 2 要素（・1個）は対象外 — 3要素以上（・2個以上）から検出。
        let two = "評価軸(精度・速度)を並べる。";
        assert!(scan(two).iter().all(|f| f.rule != "paren-nakaten"));
    }

    #[test]
    fn table_rows_and_inline_code_spans_are_excluded_like_other_detectors() {
        // 回帰（2026-07-11・R2607_026 実測で発覚）: strip_fences だけでは markdown 表の `|` 行が
        // 素通りし、セル内の「・」3連結が誤検出された。clean_lines 共有（表/inline code 除外）で
        // 是正 — readability/codemix と同じ除外契約に揃える。
        let table_row =
            "| E1 | 説明 | 吸収 | access(`adaptive_local_access`=X・`adaptive_pair_access`・Y) |";
        assert!(
            scan(table_row).iter().all(|f| f.rule != "paren-nakaten"),
            "表の行は地の文として検査してはいけない"
        );
        // inline code span 同士が「・」で並ぶ箇条書き（コード片の言及の列挙）も、
        // コード片自体が消えれば残る「・・」を実要素として誤カウントしてはいけない。
        let bullet_with_code =
            "- 事前登録: 対象は較正再生 cell(`E2607_078`・`E2607_080`・`E2607_081`)とする。";
        assert!(
            scan(bullet_with_code)
                .iter()
                .all(|f| f.rule != "paren-nakaten"),
            "inline code 言及の列挙を「・・」の空断片として誤検出した"
        );
    }

    #[test]
    fn calibration_corpus_counts_match_the_incident_report() {
        // 実測較正: 台帳体の実務文書から読者向け文書へ複写された HTML 抽出 corpus（2026-07）で
        // 矢印×18・全角イコール×10 を観測した。fixture は repo 外の一時ファイルなので、
        // ここでは代表サンプルで境界を固定する（較正の全数は README 規則台帳と実装報告に記録・
        // 本 test は回帰の pin）。
        let sample = "段階A（暫定 menu）\nmenu jump\n手順X → 手順Y は certified\n2.0〜4.7×（規模4 tapered）。\n実装込みでも 1.6〜2.2× 生存。\n稼ぐ場所の中心。";
        let hits = scan(sample);
        assert!(hits.iter().any(|f| f.rule == "arrow"));
    }
}
