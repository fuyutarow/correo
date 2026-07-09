// calque.rs — 動詞カルクの検出（英語動詞を「する/される」に直接接ぐ code-switching・2026-07-09）。
// 設計意図は qoed 由来の【狭義】: "deployする" "inspireされた" のような latin 動詞＋する接合のみ。
// 実測（raising-resolution 2026-07-09）: qoed 全 corpus で 2 件（inspireされた×2）— 頻度は低いが
// 現検出器（codemix=密度床以下・coinage=する が動詞で run 断絶・readability=非対象）を完全に素通し
// する死角であることを実証済み。広義の翻訳調（が行われ・無生物主語 等）は丁寧な人間文書と重なり
// FP 危険（が行われ×12 を実測）なので【ここでは扱わず judge の領分】— locate/judge 分業に従う。
// advisory 固定: デプロイする（カタカナ）は正当な日本語で、latin 形は register の問題 — 定着度の
// 裁定（domain/pinned/gratuitous）は codemix と同じく judge が下す。
use regex::Regex;

pub struct Finding {
    pub line: usize,
    pub word: String,
    pub msg: String,
}

/// text から latin 動詞＋する/される 接合を列挙する。fence/inline code 内は言及なので対象外。
pub fn scan(text: &str) -> Vec<Finding> {
    // latin 語（2 字以上）に する/され 系の活用が【直接】続く形。助詞を挟む正常形
    // （"ls を実行して"）は接合でないので対象外。
    let rx = Regex::new(
        r"([a-zA-Z][a-zA-Z0-9_-]+)(する|します|しました|した|して|しよう|しない|せず|され(?:る|た|て|ます)?|できる|できます)",
    )
    .unwrap();
    let inline = Regex::new(r"`[^`]*`").unwrap();
    let defenced = crate::prose::strip_fences(text);
    let mut out = Vec::new();
    for (i, raw) in defenced.lines().enumerate() {
        let clean = inline.replace_all(raw, "");
        for c in rx.captures_iter(&clean) {
            let verb = &c[1];
            let joint = &c[2];
            out.push(Finding {
                line: i + 1,
                word: format!("{verb}{joint}"),
                msg: format!(
                    "「{verb}{joint}」— 英語動詞＋{}のカルク候補。日本語動詞へ書き直すか、judge が定着語と裁定したら allow へ",
                    if joint.starts_with("され") { "される" } else { "する" }
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
    fn flags_latin_verb_suru_and_sareru_joints() {
        let v = scan("この変更を deployして、結果を mergeする。設計は inspireされた。");
        let words: Vec<&str> = v.iter().map(|f| f.word.as_str()).collect();
        assert!(words.contains(&"deployして"), "{words:?}");
        assert!(words.contains(&"mergeする"), "{words:?}");
        assert!(words.contains(&"inspireされた"), "{words:?}");
    }

    #[test]
    fn does_not_flag_normal_japanese_or_particle_separated_forms() {
        // 漢語サ変（実行する）・助詞を挟む形（ls を実行して）・カタカナ動詞化（デプロイする）は対象外。
        assert!(scan("コマンドを実行する。デプロイする手順を示す。").is_empty());
        assert!(scan("ls を実行して結果を確認する。").is_empty());
    }

    #[test]
    fn mentions_in_code_are_not_flagged() {
        assert!(scan("`deployする` は検出例。\n\n```\ndeployする\n```").is_empty());
    }
}
