// latin_token.rs — トークン単位のルー語検出（codemix/latin-token・register 軸の第五波）。
//
// 出自: 実務文書の実測で判明した codemix/latin-density の死角。長い和文の地の文へ英単語が
// 疎に埋め込まれる形（「設計後の統計リスクは baseline との比で表す。」の baseline）は、
// 段落あたりの latin/100字 密度が閾値を割るため latin-density では拾えない。実測: HTML 1 文書に
// baseline が単発挿入で 47 回出現しても、文書全体で latin-density の advisory は 1 件だけだった
// （register=consume でも変わらない — register は severity を gate するだけで、latin-density
// 自体の検出感度は上げない）。
//
// 技術文書の規約: 和文に埋め込む英単語は (a) カタカナ音訳・(b) 和訳・(c) 等幅で区切った
// code/記号 のいずれかであるべきで、地の文に出る素の小文字英単語はそのどれでもない欠陥である。
//
// 検出: prose 単位内で連続する小文字英単語の run を探す。複数語の連続 run は 1 finding
// （「shot allocation」は 2 finding でなく 1 finding）。
//
// register 軸（既存 enum と同じ・pipeline.rs 参照）:
//   - consume  → error（結論を消費するだけの読者に和文へ埋め込まれた素の英単語を渡さない）
//   - practice → advisory（実務者向けは注意喚起で足りる）
//   - internal → 発火しない（同じ語彙を共有するチーム内は codemix/latin-density だけが働く）
//
// 除外（この順で適用。いずれか一つでも該当すれば対象外）:
//   1. code span / backtick / 等幅 masked 領域 — prose.rs の既存 masking を再利用する
//      （clean_lines が fence/inline code/構造行/URL/ledger id を既に空行化・除去する）。
//   2. ALLCAPS 略語（長さ 2–6・SDP/POVM/SIC 等）。
//   3. 数字・アンダースコア・混在大小文字を含む識別子（T1・c_iid・ReplayReport）。
//   4. URL・file path・DOI（clean_lines が URL を既に strip 済みだが、file path/DOI の
//      形（スラッシュ・ドット連結を含む token）もここで追加除外する）。
//   5. allow-vocabulary 語彙表に登録済みの語。
//
// 判定基準は register + 除外規則のみで決まる — 語彙表（built-in katakana table）の membership は
// suggestion 文言の enrichment にだけ使い、発火の有無には一切影響しない（要件どおり）。
use regex::Regex;

use crate::pipeline::Register;

pub struct Finding {
    pub line: usize,
    pub rule: &'static str,
    pub msg: String,
}

/// 一般語 → 標準カタカナ音訳の built-in table（50–100 語・汎用語のみ・domain jargon は含まない）。
/// suggestion 文言の enrichment 専用 — 発火判定には影響しない（表になくても違反は成立する）。
/// アルファベット順。
pub const KATAKANA_TABLE: &[(&str, &str)] = &[
    ("access", "アクセス"),
    ("agenda", "アジェンダ"),
    ("allocation", "アロケーション"),
    ("approach", "アプローチ"),
    ("archive", "アーカイブ"),
    ("baseline", "ベースライン"),
    ("batch", "バッチ"),
    ("benchmark", "ベンチマーク"),
    ("bias", "バイアス"),
    ("boundary", "バウンダリ"),
    ("budget", "バジェット"),
    ("cache", "キャッシュ"),
    ("campaign", "キャンペーン"),
    ("capacity", "キャパシティ"),
    ("checklist", "チェックリスト"),
    ("cluster", "クラスタ"),
    ("collective", "集団"),
    ("compliance", "コンプライアンス"),
    ("component", "コンポーネント"),
    ("context", "コンテキスト"),
    ("coverage", "カバレッジ"),
    ("dashboard", "ダッシュボード"),
    ("dataset", "データセット"),
    ("default", "デフォルト"),
    ("deploy", "デプロイ"),
    ("directive", "ディレクティブ"),
    ("efficiency", "効率"),
    ("endpoint", "エンドポイント"),
    ("engine", "エンジン"),
    ("feedback", "フィードバック"),
    ("framework", "フレームワーク"),
    ("gateway", "ゲートウェイ"),
    ("guideline", "ガイドライン"),
    ("handoff", "ハンドオフ"),
    ("impact", "インパクト"),
    ("incident", "インシデント"),
    ("index", "インデックス"),
    ("infrastructure", "インフラ"),
    ("insight", "インサイト"),
    ("instance", "インスタンス"),
    ("interface", "インターフェース"),
    ("inventory", "インベントリ"),
    ("issue", "イシュー"),
    ("iteration", "イテレーション"),
    ("latency", "レイテンシ"),
    ("layer", "レイヤー"),
    ("ledger", "台帳"),
    ("lifecycle", "ライフサイクル"),
    ("margin", "マージン"),
    ("measurement", "測定"),
    ("menu", "メニュー"),
    ("metric", "メトリクス"),
    ("milestone", "マイルストーン"),
    ("module", "モジュール"),
    ("monitor", "モニター"),
    ("offset", "オフセット"),
    ("outcome", "アウトカム"),
    ("overhead", "オーバーヘッド"),
    ("owner", "オーナー"),
    ("pipeline", "パイプライン"),
    ("platform", "プラットフォーム"),
    ("policy", "ポリシー"),
    ("portfolio", "ポートフォリオ"),
    ("priority", "優先度"),
    ("protocol", "プロトコル"),
    ("provider", "プロバイダ"),
    ("proxy", "プロキシ"),
    ("query", "クエリ"),
    ("quota", "クォータ"),
    ("registry", "レジストリ"),
    ("resilience", "レジリエンス"),
    ("resource", "リソース"),
    ("roadmap", "ロードマップ"),
    ("rollback", "ロールバック"),
    ("rollout", "ロールアウト"),
    ("sampling", "サンプリング"),
    ("scaffold", "足場"),
    ("schedule", "スケジュール"),
    ("scope", "スコープ"),
    ("session", "セッション"),
    ("shot", "ショット"),
    ("snapshot", "スナップショット"),
    ("stakeholder", "ステークホルダー"),
    ("standard", "標準"),
    ("status", "ステータス"),
    ("strategy", "戦略"),
    ("summary", "サマリー"),
    ("survey", "サーベイ"),
    ("template", "テンプレート"),
    ("threshold", "閾値"),
    ("throughput", "スループット"),
    ("timeline", "タイムライン"),
    ("toolchain", "ツールチェーン"),
    ("tradeoff", "トレードオフ"),
    ("update", "アップデート"),
    ("upstream", "アップストリーム"),
    ("vendor", "ベンダー"),
    ("version", "バージョン"),
    ("workflow", "ワークフロー"),
    ("workload", "ワークロード"),
];

/// KATAKANA_TABLE から語を引く（小文字化して比較）。無ければ None。
fn katakana_for(word: &str) -> Option<&'static str> {
    let lower = word.to_lowercase();
    KATAKANA_TABLE
        .iter()
        .find(|(w, _)| *w == lower)
        .map(|(_, k)| *k)
}

/// ALLCAPS 略語（長さ 2–6・SDP/POVM/SIC 等）。
fn is_acronym(t: &str) -> bool {
    let n = t.chars().count();
    (2..=6).contains(&n) && t.chars().all(|c| c.is_ascii_uppercase())
}

/// 数字・アンダースコア・混在大小文字を含む識別子（T1・c_iid・ReplayReport）。
/// 「連続する小文字英単語」を集める前段で 1 token ごとに弾く条件（run 判定の外で使う）。
fn is_identifier_like(t: &str) -> bool {
    let has_digit = t.chars().any(|c| c.is_ascii_digit());
    let has_underscore = t.contains('_');
    let mixed_case =
        t.chars().any(|c| c.is_ascii_uppercase()) && t.chars().any(|c| c.is_ascii_lowercase());
    has_digit || has_underscore || mixed_case
}

/// 小文字のみの英単語（run 候補の要素）か。1 文字語（変数 x・数式破片）は対象外。
fn is_lowercase_word(t: &str) -> bool {
    t.chars().count() >= 2 && t.chars().all(|c| c.is_ascii_lowercase())
}

/// URL・file path・DOI らしい token（スラッシュ・ドット連結・doi: プレフィクス等）を除外する。
/// clean_lines が `https?://\S+` を既に strip 済みだが、`example.com/path` のような
/// scheme 省略形や DOI（`10.1234/...`）はここで拾う。判定は「latin span を中心に、隣接する
/// 非空白文字まで含めた周辺文字列」に対して行う（下記 surrounding_token 参照）。
fn looks_like_url_or_path_or_doi(surrounding: &str) -> bool {
    surrounding.contains('/')
        || surrounding.contains("://")
        || surrounding.to_lowercase().starts_with("doi:")
        || (surrounding.matches('.').count() >= 2
            && surrounding.contains(|c: char| c.is_ascii_digit()))
}

/// line 中の byte 位置 [start, end) を持つ latin span の周辺 token（前後の非空白文字も含めた
/// ひと続きの非空白 run）を返す。URL/path/DOI 判定は「baseline」のような孤立語でなく
/// 「docs/handbook/taxonomy.md」のように英数字以外の記号（`/` `.` `:`）を伴う周辺全体を
/// 見る必要があるための helper。
fn surrounding_token(line: &str, start: usize, end: usize) -> &str {
    let before = line[..start]
        .char_indices()
        .rev()
        .take_while(|(_, c)| !c.is_whitespace())
        .last()
        .map(|(i, _)| i)
        .unwrap_or(start);
    let after = line[end..]
        .char_indices()
        .take_while(|(_, c)| !c.is_whitespace())
        .last()
        .map(|(i, c)| end + i + c.len_utf8())
        .unwrap_or(end);
    &line[before..after]
}

/// 1 行から「連続する小文字英単語の run」を抽出する。まず ASCII 英字（＋ハイフン）の
/// maximal span を line 全体から直接拾う（空白分割に頼らない ── 和文が空白なしで英単語に
/// 隣接する「との比で表す。shot」のような token を素朴に trim すると和文接頭辞が消えて
/// 偽の run 接続が生じるため）。span 間が単純な空白（改行を含まない ASCII/全角スペース）
/// だけで隔てられていれば同一 run として連結し、それ以外（和文・記号・句読点を挟む）が
/// 挟まれば run を切る。
fn latin_runs(line: &str, exempt_lower: &std::collections::HashSet<String>) -> Vec<Vec<String>> {
    let span_re = Regex::new(r"[A-Za-z][A-Za-z-]*").unwrap();
    let mut runs: Vec<Vec<String>> = Vec::new();
    let mut cur: Vec<String> = Vec::new();
    let mut prev_end: Option<usize> = None; // 直前 span の終端 byte 位置（run 連結判定用）

    for m in span_re.find_iter(line) {
        let core = m.as_str().to_string();
        let start = m.start();
        let end = m.end();

        // run の連結可否: 直前 span の終端から今回の開始までが「空白だけ（0 文字含む）」なら
        // 連結対象、それ以外の文字（和文・句読点・括弧等）が挟まれば新しい run として切る。
        let contiguous = match prev_end {
            Some(p) => line[p..start].chars().all(|c| c == ' ' || c == '\u{3000}'),
            None => false,
        };
        if !contiguous && !cur.is_empty() {
            runs.push(std::mem::take(&mut cur));
        }
        prev_end = Some(end);

        let surrounding = surrounding_token(line, start, end);
        // is_identifier_like は周辺 token（アンダースコア・数字は ASCII 英字 span の外側に
        // あるため core だけでは見えない ── `c_iid` の `_iid` 部分・`T1` の `1` 部分）で判定する。
        let excluded = looks_like_url_or_path_or_doi(surrounding)
            || is_acronym(&core)
            || is_identifier_like(surrounding)
            || !is_lowercase_word(&core)
            || exempt_lower.contains(&core.to_lowercase());
        if excluded {
            if !cur.is_empty() {
                runs.push(std::mem::take(&mut cur));
            }
            // 除外された span 自体は run に入らないので、次の span との連結判定用に
            // prev_end は保持しつつ「現在 run は途切れている」状態（cur 空）にしておく。
            continue;
        }
        cur.push(core);
    }
    if !cur.is_empty() {
        runs.push(cur);
    }
    runs
}

/// 文書 → codemix/latin-token の全 finding。register=Internal では常に空（呼び出し側が
/// register を見て早期 return する契約は他の register-aware 検出器 — jargon.rs — と同じ）。
/// allow は許容語彙（correo.toml の allow ∪ --allow-vocabulary 由来。小文字化して比較する
/// 契約は codemix::domain_vocab と同じ）。
pub fn scan(
    text: &str,
    register: Register,
    allow: &std::collections::HashSet<String>,
) -> Vec<Finding> {
    if register == Register::Internal {
        return Vec::new();
    }
    let cleaned = crate::prose::clean_lines(text);
    let mut out = Vec::new();
    for (i, line) in cleaned.lines().enumerate() {
        let lineno = i + 1;
        for run in latin_runs(line, allow) {
            let phrase = run.join(" ");
            let suggestion = if run.len() == 1 {
                match katakana_for(&run[0]) {
                    Some(k) => format!("{k} へ"),
                    None => "カタカナ語化・和訳・等幅化のいずれかへ".to_string(),
                }
            } else {
                let hints: Vec<String> = run
                    .iter()
                    .filter_map(|w| katakana_for(w).map(|k| format!("{w}→{k}")))
                    .collect();
                if hints.is_empty() {
                    "カタカナ語化・和訳・等幅化のいずれかへ".to_string()
                } else {
                    format!(
                        "{} 等へ（カタカナ語化・和訳・等幅化のいずれか）",
                        hints.join("・")
                    )
                }
            };
            out.push(Finding {
                line: lineno,
                rule: "latin-token",
                msg: format!(
                    "「{phrase}」 is a bare english {} embedded in japanese prose — {suggestion}",
                    if run.len() == 1 { "word" } else { "phrase" }
                ),
            });
        }
    }
    out
}

/// register から severity 文字列（"error"/"advisory"）を決める。pipeline.rs 側の
/// Finding 組み立てで使う（jargon.rs の advisory 固定と異なり本規則は register で severity が
/// 変わるため、pipeline 側に register を渡す代わりにここで決定関数を公開する）。
pub fn severity_for(register: Register) -> &'static str {
    match register {
        Register::Consume => "error",
        Register::Practice => "advisory",
        Register::Internal => "advisory", // 呼ばれない想定（scan が空を返すため無害な既定）
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const REPRO: &str = "設計後の統計リスクは baseline との比で表す。shot allocation を最適化すると、collective measurement の理論限界に近づく。";

    fn allow() -> std::collections::HashSet<String> {
        std::collections::HashSet::new()
    }

    #[test]
    fn internal_register_never_fires() {
        assert!(scan(REPRO, Register::Internal, &allow()).is_empty());
    }

    #[test]
    fn consume_register_finds_three_findings_for_repro_sentence() {
        let hits = scan(REPRO, Register::Consume, &allow());
        assert_eq!(
            hits.len(),
            3,
            "3 finding のはず: {:?}",
            hits.iter().map(|f| &f.msg).collect::<Vec<_>>()
        );
        assert!(hits.iter().any(|f| f.msg.contains("baseline")));
        assert!(hits.iter().any(|f| f.msg.contains("shot allocation")));
        assert!(
            hits.iter()
                .any(|f| f.msg.contains("collective measurement"))
        );
    }

    #[test]
    fn practice_register_finds_same_three_findings() {
        let hits = scan(REPRO, Register::Practice, &allow());
        assert_eq!(hits.len(), 3);
    }

    #[test]
    fn consecutive_latin_words_merge_into_one_finding_not_two() {
        let hits = scan(
            "shot allocation を最適化する。",
            Register::Consume,
            &allow(),
        );
        assert_eq!(
            hits.len(),
            1,
            "1 finding のはず: {:?}",
            hits.iter().map(|f| &f.msg).collect::<Vec<_>>()
        );
        assert!(hits[0].msg.contains("shot allocation"));
    }

    #[test]
    fn severity_maps_consume_to_error_and_practice_to_advisory() {
        assert_eq!(severity_for(Register::Consume), "error");
        assert_eq!(severity_for(Register::Practice), "advisory");
    }

    #[test]
    fn allcaps_acronym_does_not_fire() {
        let hits = scan("この設計は POVM を使う。", Register::Consume, &allow());
        assert!(
            hits.is_empty(),
            "POVM が発火した: {:?}",
            hits.iter().map(|f| &f.msg).collect::<Vec<_>>()
        );
    }

    #[test]
    fn short_and_long_allcaps_boundary() {
        // 2–6 字の ALLCAPS は略語。7 字以上は略語扱いしない（境界を明示的に固定）。
        let hits = scan("この SIC と SDP を比較する。", Register::Consume, &allow());
        assert!(hits.is_empty());
    }

    #[test]
    fn identifier_with_digit_does_not_fire() {
        let hits = scan("この T1 の緩和時間を測る。", Register::Consume, &allow());
        assert!(
            hits.is_empty(),
            "T1 が発火した: {:?}",
            hits.iter().map(|f| &f.msg).collect::<Vec<_>>()
        );
    }

    #[test]
    fn identifier_with_underscore_does_not_fire() {
        let hits = scan("この c_iid を評価する。", Register::Consume, &allow());
        assert!(
            hits.is_empty(),
            "c_iid が発火した: {:?}",
            hits.iter().map(|f| &f.msg).collect::<Vec<_>>()
        );
    }

    #[test]
    fn mixed_case_identifier_does_not_fire() {
        let hits = scan(
            "この ReplayReport を確認する。",
            Register::Consume,
            &allow(),
        );
        assert!(
            hits.is_empty(),
            "ReplayReport が発火した: {:?}",
            hits.iter().map(|f| &f.msg).collect::<Vec<_>>()
        );
    }

    #[test]
    fn backtick_code_span_does_not_fire() {
        let text = "`baseline` は識別子の言及。地の文には無い。";
        let hits = scan(text, Register::Consume, &allow());
        assert!(
            hits.is_empty(),
            "backtick 内が発火した: {:?}",
            hits.iter().map(|f| &f.msg).collect::<Vec<_>>()
        );
    }

    #[test]
    fn fence_code_block_does_not_fire() {
        let text = "説明文。\n\n```\nbaseline = compute_baseline()\n```\n";
        let hits = scan(text, Register::Consume, &allow());
        assert!(
            hits.is_empty(),
            "fence 内が発火した: {:?}",
            hits.iter().map(|f| &f.msg).collect::<Vec<_>>()
        );
    }

    #[test]
    fn allow_vocabulary_word_does_not_fire() {
        let mut a = allow();
        a.insert("baseline".to_string());
        let hits = scan(REPRO, Register::Consume, &a);
        assert!(
            !hits.iter().any(|f| f.msg.contains("「baseline」")),
            "allow 登録語が発火した: {:?}",
            hits.iter().map(|f| &f.msg).collect::<Vec<_>>()
        );
        // 他の 2 件は引き続き発火する。
        assert_eq!(hits.len(), 2);
    }

    #[test]
    fn url_is_excluded() {
        let text = "詳細は https://example.com/path/to/page を参照。";
        let hits = scan(text, Register::Consume, &allow());
        assert!(
            hits.is_empty(),
            "URL が発火した: {:?}",
            hits.iter().map(|f| &f.msg).collect::<Vec<_>>()
        );
    }

    #[test]
    fn schemeless_path_is_excluded() {
        let text = "設定は docs/handbook/taxonomy.md にある。";
        let hits = scan(text, Register::Consume, &allow());
        assert!(
            hits.is_empty(),
            "path が発火した: {:?}",
            hits.iter().map(|f| &f.msg).collect::<Vec<_>>()
        );
    }

    #[test]
    fn doi_is_excluded() {
        let text = "出典は doi:10.1234/example.5678 である。";
        let hits = scan(text, Register::Consume, &allow());
        assert!(
            hits.is_empty(),
            "DOI が発火した: {:?}",
            hits.iter().map(|f| &f.msg).collect::<Vec<_>>()
        );
    }

    #[test]
    fn suggestion_uses_katakana_table_entry_when_present() {
        let hits = scan("これは baseline である。", Register::Consume, &allow());
        assert!(
            hits.iter().any(|f| f.msg.contains("ベースライン")),
            "katakana table の候補語が suggestion に出ない: {:?}",
            hits.iter().map(|f| &f.msg).collect::<Vec<_>>()
        );
    }

    #[test]
    fn suggestion_falls_back_to_generic_hint_when_word_not_in_table() {
        let hits = scan("これは zorptastic である。", Register::Consume, &allow());
        assert!(
            hits.iter()
                .any(|f| f.msg.contains("カタカナ語化・和訳・等幅化のいずれかへ")),
            "表に無い語で汎用 suggestion が出ない: {:?}",
            hits.iter().map(|f| &f.msg).collect::<Vec<_>>()
        );
    }

    #[test]
    fn table_membership_does_not_gate_detection_only_enriches_message() {
        // zorptastic は表に無いが、それでも violation は成立する（要件: 表は発火条件に無関係）。
        let hits = scan("これは zorptastic である。", Register::Consume, &allow());
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn single_char_math_fragment_does_not_fire() {
        let hits = scan("O(n) の x を評価する。", Register::Consume, &allow());
        assert!(
            hits.is_empty(),
            "数式破片が発火した: {:?}",
            hits.iter().map(|f| &f.msg).collect::<Vec<_>>()
        );
    }

    #[test]
    fn three_word_run_is_still_a_single_finding() {
        let hits = scan(
            "quantum error correction を導入する。",
            Register::Consume,
            &allow(),
        );
        assert_eq!(
            hits.len(),
            1,
            "{:?}",
            hits.iter().map(|f| &f.msg).collect::<Vec<_>>()
        );
        assert!(hits[0].msg.contains("quantum error correction"));
    }

    #[test]
    fn katakana_table_has_no_domain_jargon_only_general_words() {
        // 要件: 50–100 語・汎用語のみ（ドメイン jargon を含まない）。件数レンジだけを固定する
        // （語彙の中身の妥当性は目視裁定 — 本 test は範囲の回帰）。
        assert!(
            KATAKANA_TABLE.len() >= 50 && KATAKANA_TABLE.len() <= 100,
            "table サイズが範囲外: {}",
            KATAKANA_TABLE.len()
        );
    }
}
