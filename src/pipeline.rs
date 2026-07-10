// pipeline.rs — check の検査エンジン（検出器の registry ＋ 収集の単一 loop）。
// 2026-07-09 抽出: main.rs の Check アームが 8 検出器を個別 block で手配線し、
// 「scan → suppress 除外 → file 刻印 → push」を 8 回複写していた（divergent-change smell）。
// その protocol を scan_document の 1 loop へ集約し、各検出器の raw 型→Finding 変換を
// ここへ寄せた。CLI（main.rs）は arg 解釈と出力描画に責務を絞る。
// coinage は横断 batch＋corpus 昇格＋feature-gate ゆえ本 loop に載せず main.rs が持つ（無理に畳まない）。
use crate::report::{Finding, Severity, Violation};
use crate::{
    calque, codemix, completeness, counter, density, deny, jargon, notation, readability, rhetoric,
    structure, suppress,
};
use std::collections::{HashMap, HashSet};

/// 読者軸（register）。既定は Internal（現行動作＝不変）。Practice/Consume のときだけ
/// jargon-export が発火する ── 同じ語彙表（taxonomy.md 等）が、Internal では免除リスト
/// （書き手の語彙 = 許される語彙）、Practice/Consume では検査対象語彙（読者と共有されていない
/// 内輪語の候補）として役割を反転する。
/// 出自: 台帳体の統制語彙表を持つ実務プロジェクトの実測事例（2026-07・campaign・criterion 族・
/// menu 級の内輪語が broad readership へ定義なしで輸出された）。
///
/// Practice と Consume の分岐（第四波・同日追記）: 「この語彙をこの後使う読者」（Practice —
/// 実務者向け仕様書。語を習得して使いこなす前提）と「結論を消費する読者」（Consume — 概況・
/// 報告の読み手。語を習得する必要が無く、させてもいけない）は合格条件が逆になる。Practice は
/// 「定義があれば合格」（初出定義または用語表）。Consume は「地の文に出てこなければ合格」 ──
/// 定義や用語表があっても、語彙表の語が散文単位に出現した時点でそれ自体が advisory 違反になる。
/// 用語表を建てる解は「読者に書き手の語彙を習得させる」誤りであり、消費型読者には不出現こそが
/// 正しい合格条件（用語表は解でなく症状）というのが本 register の判定基準である。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Register {
    #[default]
    Internal,
    /// 旧 External の改名（後方互換で `external` は本値の別名として CLI/toml から受理する）。
    /// 語彙表の語は初出定義（近傍言い換え・とは文）または文書内の用語表があれば合格。
    Practice,
    /// 新設（2026-07-11・第四波）。語彙表の語が地の文（コードフェンス・インライン
    /// コード・等幅の識別子文脈を除く）に出現したら、それ自体が advisory 違反。
    /// 定義や用語表があっても合格にしない。
    Consume,
}

/// 検出器が要する文脈（config 由来）。文書に依らないので 1 度組んで各文書へ貸す。
pub struct Ctx<'a> {
    pub max_sentence: usize,
    pub max_ten: usize,
    pub threshold: f64,
    pub exempt: &'a HashSet<String>,
    pub allow_set: &'a HashSet<&'a str>,
    pub metaphor_lex: &'a [(String, String)],
    pub user_deny: &'a HashMap<String, String>,
    pub builtin_deny: &'a HashMap<String, String>,
    pub register: Register,
    /// jargon-export の検査対象語彙（register=External のときだけ使う。jargon::load_terms が
    /// 語彙表から第 1 列語を抽出したもの）。Internal では無視される。
    pub jargon_terms: &'a [String],
}

fn sev(s: Severity) -> &'static str {
    match s {
        Severity::Hard => "error",
        Severity::Advisory => "advisory",
    }
}

/// report::Violation を返す検出器（readability / rhetoric / structure）共通の adapter。
/// severity は Violation 由来（readability は Hard/Advisory 混在・他は全 Advisory）。
fn from_violations(vios: Vec<Violation>, detector: &'static str) -> Vec<Finding> {
    vios.into_iter()
        .map(|v| Finding {
            detector,
            rule: v.rule.into(),
            file: String::new(),
            line: v.line,
            severity: sev(v.severity),
            message: v.msg,
            data: None,
        })
        .collect()
}

fn deny_findings(
    text: &str,
    map: &HashMap<String, String>,
    rule: &'static str,
    mk: impl Fn(&str, &str) -> String,
) -> Vec<Finding> {
    deny::scan(text, map)
        .into_iter()
        .map(|(line, w, sugg)| {
            let message = mk(&w, &sugg);
            Finding {
                detector: "deny",
                rule: rule.into(),
                file: String::new(),
                line,
                severity: "error",
                message,
                data: Some(serde_json::json!({ "word": w, "suggestion": sugg })),
            }
        })
        .collect()
}

fn codemix_findings(text: &str, exempt: &HashSet<String>, threshold: f64) -> Vec<Finding> {
    codemix::scan_paragraphs(text, exempt)
        .into_iter()
        .filter(|p| p.density >= threshold)
        .map(|p| Finding {
            detector: "codemix",
            rule: "latin-density".into(),
            file: String::new(),
            line: p.line,
            severity: "advisory",
            message: format!(
                "{:.0} latin per 100 JA chars (JA {}) — judge each token: domain→allow / mention→backticks / gratuitous→rewrite: {}",
                p.density,
                p.ja_chars,
                p.vocab.join(" ")
            ),
            data: Some(serde_json::json!({
                "density": p.density,
                "ja_chars": p.ja_chars,
                "vocab": p.vocab,
            })),
        })
        .collect()
}

fn calque_findings(text: &str) -> Vec<Finding> {
    calque::scan(text)
        .into_iter()
        .map(|c| Finding {
            detector: "calque",
            rule: "verb-calque".into(),
            file: String::new(),
            line: c.line,
            severity: "advisory",
            message: c.msg,
            data: Some(serde_json::json!({ "word": c.word })),
        })
        .collect()
}

fn density_findings(text: &str) -> Vec<Finding> {
    density::scan(text)
        .into_iter()
        .map(|d| Finding {
            detector: "density",
            rule: d.rule.into(),
            file: String::new(),
            line: d.line,
            severity: "advisory",
            message: d.msg,
            data: None,
        })
        .collect()
}

fn notation_findings(text: &str) -> Vec<Finding> {
    notation::scan(text)
        .into_iter()
        .map(|n| Finding {
            detector: "notation",
            rule: n.rule.into(),
            file: String::new(),
            line: n.line,
            severity: "advisory",
            message: n.msg,
            data: None,
        })
        .collect()
}

fn completeness_findings(text: &str) -> Vec<Finding> {
    completeness::scan(text)
        .into_iter()
        .map(|c| Finding {
            detector: "completeness",
            rule: c.rule.into(),
            file: String::new(),
            line: c.line,
            severity: "advisory",
            message: c.msg,
            data: None,
        })
        .collect()
}

fn counter_findings(text: &str) -> Vec<Finding> {
    counter::scan(text)
        .into_iter()
        .map(|c| Finding {
            detector: "counter",
            rule: c.rule.into(),
            file: String::new(),
            line: c.line,
            severity: "advisory",
            message: c.msg,
            data: None,
        })
        .collect()
}

/// jargon-export は register=Practice/Consume のときだけ発火する（Internal では同じ語彙表が
/// 免除リストとして働く既存動作のまま・不変）。raw は用語表（markdown table / HTML table・dl）の
/// 列構造を判定するための構造保持テキスト（呼び出し側が strip_html 前のソースを渡す）。
/// Practice/Consume で判定基準を切り替える（jargon::scan/scan_consume・詳細は jargon.rs 冒頭）。
fn jargon_findings(raw: &str, text: &str, ctx: &Ctx) -> Vec<Finding> {
    let clean = crate::prose::clean_lines(text);
    let hits = match ctx.register {
        Register::Internal => return Vec::new(),
        Register::Practice => jargon::scan(raw, &clean, ctx.jargon_terms),
        Register::Consume => jargon::scan_consume(raw, &clean, ctx.jargon_terms),
    };
    hits.into_iter()
        .map(|j| Finding {
            detector: "jargon",
            rule: j.rule.into(),
            file: String::new(),
            line: j.line,
            severity: "advisory",
            message: j.msg,
            data: None,
        })
        .collect()
}

/// 1 文書へ全 per-file 検出器を registry 順に走らせ、suppress を適用し file を刻んで Finding を返す。
/// 検出器の追加はこの配列に 1 行足すだけ（main の CLI アームは編集しない）。順序＝出力順（push 順・
/// report は sort しない）なので registry の並びを変えると出力が変わる — golden で固定。
/// raw は jargon-export の用語表判定専用（text と同一内容で構わない ── strip_html を通していない
/// markdown 呼び出し側では text と raw は同じ文字列を渡せばよい。HTML 呼び出し側は strip_html 前の
/// ソースを raw に、strip_html 後を text に渡す）。
pub fn scan_document(
    file: &str,
    text: &str,
    raw: &str,
    s: &suppress::Suppressions,
    ctx: &Ctx,
) -> Vec<Finding> {
    let groups: Vec<Vec<Finding>> = vec![
        deny_findings(text, ctx.user_deny, "denied-term", |w, sugg| {
            format!("「{w}」 is a denied term (judge's recorded verdict) — {sugg}")
        }),
        deny_findings(text, ctx.builtin_deny, "slop-phrase", |w, sugg| {
            format!(
                "「{w}…」 is an LLM stock phrase (builtin deny) — {sugg} (unblock: add to allow with a reason)"
            )
        }),
        codemix_findings(text, ctx.exempt, ctx.threshold),
        from_violations(
            readability::scan(text, ctx.max_sentence, ctx.max_ten),
            "readability",
        ),
        from_violations(
            rhetoric::scan(text, ctx.allow_set, ctx.metaphor_lex),
            "rhetoric",
        ),
        from_violations(structure::scan(text), "structure"),
        calque_findings(text),
        density_findings(text),
        notation_findings(text),
        completeness_findings(text),
        counter_findings(text),
        jargon_findings(raw, text, ctx),
    ];
    let mut out = Vec::new();
    for raws in groups {
        for mut fi in raws {
            if s.hit(fi.line, fi.detector, &fi.rule) {
                continue;
            }
            fi.file = file.to_string();
            out.push(fi);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_ctx<'a>(
        exempt: &'a HashSet<String>,
        allow_set: &'a HashSet<&'a str>,
        metaphor_lex: &'a [(String, String)],
        user_deny: &'a HashMap<String, String>,
        builtin_deny: &'a HashMap<String, String>,
        register: Register,
        jargon_terms: &'a [String],
    ) -> Ctx<'a> {
        Ctx {
            max_sentence: 100,
            max_ten: 4,
            threshold: 8.0,
            exempt,
            allow_set,
            metaphor_lex,
            user_deny,
            builtin_deny,
            register,
            jargon_terms,
        }
    }

    #[test]
    fn jargon_export_is_silent_under_internal_register_even_with_terms_configured() {
        // internal（既定）は jargon-export を発火させない ── 同じ語彙表が免除リストとして
        // 働く既存動作は不変であるべき（register 軸導入の中心契約）。
        let exempt = HashSet::new();
        let allow_set = HashSet::new();
        let metaphor_lex = vec![];
        let user_deny = HashMap::new();
        let builtin_deny = HashMap::new();
        let terms = vec!["campaign".to_string()];
        let ctx = base_ctx(
            &exempt,
            &allow_set,
            &metaphor_lex,
            &user_deny,
            &builtin_deny,
            Register::Internal,
            &terms,
        );
        let text = "campaign を設計する。";
        let s = suppress::scan(text);
        let findings = scan_document("doc.md", text, text, &s, &ctx);
        assert!(
            findings.iter().all(|f| f.detector != "jargon"),
            "internal で jargon-export が発火した: {:?}",
            findings.iter().map(|f| &f.rule).collect::<Vec<_>>()
        );
    }

    #[test]
    fn jargon_export_fires_under_practice_register_and_is_suppressible() {
        let exempt = HashSet::new();
        let allow_set = HashSet::new();
        let metaphor_lex = vec![];
        let user_deny = HashMap::new();
        let builtin_deny = HashMap::new();
        let terms = vec!["campaign".to_string()];
        let ctx = base_ctx(
            &exempt,
            &allow_set,
            &metaphor_lex,
            &user_deny,
            &builtin_deny,
            Register::Practice,
            &terms,
        );
        let text = "campaign を設計する。";
        let s = suppress::scan(text);
        let findings = scan_document("doc.md", text, text, &s, &ctx);
        assert!(
            findings
                .iter()
                .any(|f| f.detector == "jargon" && f.rule == "jargon-export"),
            "practice で jargon-export が発火しなかった"
        );

        // inline 抑制（<!-- correo-ignore jargon -->）が他検出器と同じ経路で効くこと。
        let suppressed_text = "campaign を設計する。 <!-- correo-ignore jargon -->";
        let s2 = suppress::scan(suppressed_text);
        let findings2 = scan_document("doc.md", suppressed_text, suppressed_text, &s2, &ctx);
        assert!(
            findings2.iter().all(|f| f.detector != "jargon"),
            "jargon の inline 抑制が効かなかった"
        );
    }

    #[test]
    fn jargon_export_fires_under_consume_register_even_with_glossary_definition() {
        // consume の中心契約: practice なら合格する（用語表・近傍定義がある）語でも、
        // consume では地の文出現そのものが違反になる。
        let exempt = HashSet::new();
        let allow_set = HashSet::new();
        let metaphor_lex = vec![];
        let user_deny = HashMap::new();
        let builtin_deny = HashMap::new();
        let terms = vec!["campaign".to_string()];
        let ctx = base_ctx(
            &exempt,
            &allow_set,
            &metaphor_lex,
            &user_deny,
            &builtin_deny,
            Register::Consume,
            &terms,
        );
        let text = "campaign（測定の実施計画）を設計する。";
        let s = suppress::scan(text);
        let findings = scan_document("doc.md", text, text, &s, &ctx);
        assert!(
            findings
                .iter()
                .any(|f| f.detector == "jargon" && f.rule == "jargon-export"),
            "consume では近傍定義があっても発火するはず"
        );
    }

    #[test]
    fn jargon_export_finding_is_advisory_severity() {
        let exempt = HashSet::new();
        let allow_set = HashSet::new();
        let metaphor_lex = vec![];
        let user_deny = HashMap::new();
        let builtin_deny = HashMap::new();
        let terms = vec!["campaign".to_string()];
        let ctx = base_ctx(
            &exempt,
            &allow_set,
            &metaphor_lex,
            &user_deny,
            &builtin_deny,
            Register::Practice,
            &terms,
        );
        let text = "campaign を設計する。";
        let s = suppress::scan(text);
        let findings = scan_document("doc.md", text, text, &s, &ctx);
        let j = findings
            .iter()
            .find(|f| f.detector == "jargon")
            .expect("jargon finding が無い");
        assert_eq!(j.severity, "advisory");
    }
}
