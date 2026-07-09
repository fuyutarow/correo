//! correo — 日本語実用文の lint: LLM slop ＋ 木下是雄の原則のうち機械的に判定できるもの。
//!
//! module 分割の原則（2026-07-09 再分割）: **検出器 = 検査する性質 1 つ**。規則の出自
//! （木下の章・textlint 移植・house）は module でなく規則ごとに README の規則台帳が記録する。
//! 検査する性質は 6 つ（事実性・hype は対象外）:
//! - 言語の自然さ: `codemix`（ルー語密度）・`calque`（動詞カルク: deployする）・
//!   `coinage`（Sudachi 辞書外複合 = 造語・混種語）
//! - 可読性・トーンの床: `readability`（文長・読点・文体混在・二重否定・ぼかし・指示語・
//!   の連鎖・冗長・漢字連続・感嘆符 — 木下に根拠を持つ規則だけを置く）
//! - 定型構造: `structure`（bullet-template・opener-repetition・connector-pileup・
//!   colon-continuation — LLM layout の指紋。木下と無関係ゆえ別 home）
//! - 情報密度: `density`（圧縮率 bits/字・段落近重複）
//! - 修辞密度: `rhetoric`（対立法/ダッシュ/副題統一の composite・装飾メタファー語彙 —
//!   文体の指紋であって明晰さの欠陥でない。rate 単独で発火せず、全て advisory で judge へ）
//! - 語彙の裁定: `deny`（組み込み slop 常套句＋judge 確定裁定の永続 cache）
//!
//! 判定の分業: readability の HARD と deny は機械判定が最終（blocking）。他は全て locate 層 —
//! 候補を flag し、最終判定は呼び出し側（LLM-judge か人）へ委ねる。allow は呼び出し側が注入。
pub mod calque;
pub mod codemix;
pub mod coinage;
pub mod config;
pub mod density;
pub mod deny;
pub mod prose;
pub mod readability;
pub mod report;
pub mod rhetoric;
pub mod structure;
pub mod suppress;
