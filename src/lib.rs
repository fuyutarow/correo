//! correo — 日本語実用文の lint: LLM slop ＋ 木下是雄の原則のうち機械的に判定できるもの。
//!
//! module 分割の原則（2026-07-09 再分割）: **検出器 = 検査する性質 1 つ**。規則の出自
//! （木下の章・textlint 移植・house）は module でなく規則ごとに README の規則台帳が記録する。
//! 検査する性質は 10 つ（事実性・hype は対象外）:
//! - 言語の自然さ: `codemix`（ルー語密度）・`calque`（動詞カルク: deployする）・
//!   `coinage`（Sudachi 辞書外複合 = 造語・混種語）
//! - 可読性・トーンの床: `readability`（文長・読点・文体混在・二重否定・ぼかし・指示語・
//!   の連鎖・冗長・漢字連続・感嘆符 — 木下に根拠を持つ規則だけを置く）
//! - 定型構造: `structure`（bullet-template・opener-repetition・connector-pileup・
//!   colon-continuation — LLM layout の指紋。木下と無関係ゆえ別 home）
//! - 情報密度: `density`（圧縮率 bits/字・段落近重複）
//! - 修辞密度: `rhetoric`（対立法/ダッシュ/副題統一の composite・装飾メタファー語彙 —
//!   文体の指紋であって明晰さの欠陥でない。rate 単独で発火せず、全て advisory で judge へ）
//! - 台帳記法の漏出: `notation`（矢印・全角イコール・括弧内 3 連結 — 台帳体から読者向け散文への
//!   複写事故。rhetoric の「——」密度とは別性質＝記号が文法的役割を肩代わりしているかを見る）
//! - 助数詞の欠落: `counter`（数字＋ラテン名詞が助詞・句読点に直結 — 台帳体の圧縮表記
//!   「43 cell」が読者向け散文に漏れる事故。notation とは別性質＝記号でなく数字とラテン名詞の
//!   直結そのものを見る）
//! - 文の資格: `completeness`（散文単位が文の終端記号で閉じているか。readability の文内部品質
//!   とは別性質＝終端の有無そのものを見る）
//! - 内輪語の輸出: `jargon`（register=practice/consume でだけ発火。読者と共有されていない統制
//!   語彙が定義なしで使われているかを見る ── register=internal では同じ語彙表が免除リストとして
//!   働く、の逆機能。practice=定義があれば合格・consume=地の文に出てこなければ合格）
//! - 語彙の裁定: `deny`（組み込み slop 常套句＋judge 確定裁定の永続 cache）
//!
//! 判定の分業: readability の HARD と deny は機械判定が最終（blocking）。他は全て locate 層 —
//! 候補を flag し、最終判定は呼び出し側（LLM-judge か人）へ委ねる。allow は呼び出し側が注入。
pub mod calque;
pub mod codemix;
pub mod coinage;
pub mod completeness;
pub mod config;
pub mod counter;
pub mod density;
pub mod deny;
pub mod jargon;
pub mod notation;
pub mod pipeline;
pub mod prose;
pub mod readability;
pub mod report;
pub mod rhetoric;
pub mod setup;
pub mod structure;
pub mod suppress;
