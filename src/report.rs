// report.rs — Tier 3 の judge 界面: 全検出器の finding を機械可読 JSON で emit する。
// 思想（grammar-constrained generation の正しい適用先）: 「木で縛る」のは散文でなく **judge の
// 入出力**。correo は座標（file/line/rule/data）を構造化して渡し、judge 側は structured output
// （JSON schema 制約）で 3-way 分類（domain / pinned / gratuitous 等）を返す。correo は判定しない。
// schema は追加互換で進化させる（version field・既存 key の意味は変えない）。
use serde::Serialize;

/// Hard = 機械判定が最終（blocking・exit に数える）。Advisory = 高精度 heuristic だが
/// 文脈で正当があり得る（報告のみ・judge/人の確認へ回す）。
/// 検出器横断の共通型（2026-07-09 再分割で kinoshita 専有から一段上げた — structure と共有）。
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

#[derive(Serialize)]
pub struct Finding {
    /// codemix | kinoshita | coinage
    pub detector: &'static str,
    pub rule: String,
    pub file: String,
    pub line: usize,
    /// "error"（HARD・exit に数える）| "advisory"（locate 候補・judge へ）
    pub severity: &'static str,
    pub message: String,
    /// 検出器固有の構造 data（codemix: density/vocab、coinage: compound/components 等）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[derive(Serialize)]
pub struct Report {
    pub version: u32,
    pub findings: Vec<Finding>,
    pub summary: Summary,
}

#[derive(Serialize)]
pub struct Summary {
    pub error: usize,
    pub advisory: usize,
}

impl Report {
    pub fn new(findings: Vec<Finding>) -> Self {
        let error = findings.iter().filter(|f| f.severity == "error").count();
        let advisory = findings.len() - error;
        Report {
            version: 1,
            findings,
            summary: Summary { error, advisory },
        }
    }
}
