//! ja-slop-lint — 日本語の LLM slop を検出する lint。
//!
//! 「slop」＝**言語の不自然さ**（機械が混ぜた非母語的な日本語）で、事実性・冗長性は対象外:
//! - `codemix`: 地の文の latin/100字 密度（ルー語の locate 層・識別子/ALLCAPS 略語/登録語は除外）。
//! - `coinage`: Sudachi 形態素の辞書外複合（不自然な造語）検出。
//! - （順次）`calque`: 英語動詞を する に接ぐ code-switching の検出。
//!
//! locate 層であって judge でない ── 候補を flag し、domain か gratuitous かの判定は
//! 呼び出し側（LLM-judge か人）へ委ねる。exempt 語彙（allow-list）は呼び出し側が注入する。
pub mod codemix;
pub mod coinage;
