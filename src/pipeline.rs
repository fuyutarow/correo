// pipeline.rs — check の検査エンジン（検出器の registry ＋ 収集の単一 loop）。
// 2026-07-09 抽出: main.rs の Check アームが 8 検出器を個別 block で手配線し、
// 「scan → suppress 除外 → file 刻印 → push」を 8 回複写していた（divergent-change smell）。
// その protocol を scan_document の 1 loop へ集約し、各検出器の raw 型→Finding 変換を
// ここへ寄せた。CLI（main.rs）は arg 解釈と出力描画に責務を絞る。
// coinage は横断 batch＋corpus 昇格＋feature-gate ゆえ本 loop に載せず main.rs が持つ（無理に畳まない）。
use crate::report::{Finding, Severity, Violation};
use crate::{calque, codemix, density, deny, readability, rhetoric, structure, suppress};
use std::collections::{HashMap, HashSet};

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

/// 1 文書へ全 per-file 検出器を registry 順に走らせ、suppress を適用し file を刻んで Finding を返す。
/// 検出器の追加はこの配列に 1 行足すだけ（main の CLI アームは編集しない）。順序＝出力順（push 順・
/// report は sort しない）なので registry の並びを変えると出力が変わる — golden で固定。
pub fn scan_document(
    file: &str,
    text: &str,
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
