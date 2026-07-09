//! correo — 日本語実用文の lint: LLM slop ＋ 木下是雄の機械判定可能層。
//!
//! 二軸を検査する（事実性・hype は対象外）:
//! 「slop」＝**言語の不自然さ**（機械が混ぜた非母語的な日本語）——
//! - `codemix`: 地の文の latin/100字 密度（ルー語の locate 層・識別子/ALLCAPS 略語/登録語は除外）。
//! - `coinage`: Sudachi 形態素の辞書外複合（不自然な造語）検出。
//! - （順次）`calque`: 英語動詞を する に接ぐ code-switching の検出。
//!
//! 「木下 HARD 層」＝『理科系の作文技術』の**機械が言い切れる床**（2026-07-09 スコープ拡張）——
//! - `kinoshita`: 文長・読点過多・ですます/である混在・慣用二重否定・ぼかし連発・指示語連鎖。
//!
//! slop 軸は locate 層であって judge でない ── 候補を flag し、domain か gratuitous かの判定は
//! 呼び出し側（LLM-judge か人）へ委ねる。exempt 語彙（allow-list）は呼び出し側が注入する。
//! kinoshita 軸だけは HARD（機械判定が最終）ゆえ blocking 可。
pub mod codemix;
pub mod coinage;
pub mod kinoshita;
pub mod prose;
pub mod report;
