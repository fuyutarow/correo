// calque.rs — 動詞カルクの検出（英語動詞を「する/される」に直接接ぐ code-switching・2026-07-09）。
// 設計意図は実務プロジェクトでの実測に基づく【狭義】: "deployする" "inspireされた" のような
// latin 動詞＋する接合のみ。実測（raising-resolution 2026-07-09）: 実務文書 corpus で 2 件
// （inspireされた×2）— 頻度は低いが現検出器（codemix=密度床以下・coinage=する が動詞で run
// 断絶・readability=非対象）を完全に素通しする死角であることを実証済み。広義の翻訳調（が行われ・
// 無生物主語 等）は丁寧な人間文書と重なり FP 危険（が行われ×12 を実測）なので【ここでは扱わず
// judge の領分】— locate/judge 分業に従う。
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
    // 小文字始まりの latin 語（≥2 字）に する/され 系の活用が続く形。dotfiles hook の
    // session-hardened regex を移植（2026-07-09・単一 source of truth 化）:
    //   - 小文字始まり限定 ⇒ 英語動詞（commit/cite）のみ・固有名（GitHub/AWS）は除外。
    //   - 活用直前の空白は許す（` *`）⇒ 「flag された」を捕捉（された gap で 1 session 素通りした）。
    //   - 活用は voice/tense/aspect を網羅（させ*/したい/したく を含む — improveさせたい gap）。
    //   - 助詞を挟む正常形（「ls を実行して」「commit を実行する」）は を が割るので対象外。
    let rx = Regex::new(
        r"(?:^|[^A-Za-z])([a-z][a-zA-Z-]+) *(する|します|した|して|している|していた|しています|しており|される|された|されて|されない|されました|できる|できた|できない|しない|しなかった|せず|しよう|すれば|すべき|しろ|せよ|させる|させます|させた|させて|させている|させたい|させない|させず|させよう|したい|したく|したければ)",
    )
    .unwrap();
    let inline = Regex::new(r"`[^`]*`").unwrap();
    let defenced = crate::prose::strip_source_blocks(text);
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
                    "「{verb}{joint}」 — english verb spliced onto 「{}」 (calque candidate) — write the Japanese verb, or add to allow if judge rules it established",
                    if joint.starts_with("され") { "される" } else { "する" }
                ),
            });
        }
    }
    out
}

/// CLI entry（stdin か file 群を読み、カルク候補を列挙）。hook のバックエンド用に
/// 単体で呼べる — `check` は *.md file を要するが、Stop hook が持つのは turn の text なので
/// stdin 経路が要る（codemix/coinage subcommand と同型）。hit があれば exit 1（--advisory で 0）。
pub fn run_calque(files: &[String], advisory: bool) -> i32 {
    use std::io::Read;
    let mut n = 0usize;
    let mut report = |label: &str, text: &str| {
        for f in scan(text) {
            n += 1;
            println!("{label}L{}: [calque/verb-calque] {}", f.line, f.msg);
        }
    };
    if files.is_empty() {
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf).ok();
        report("", &buf);
    } else {
        for f in files {
            match std::fs::read_to_string(f) {
                Err(e) => eprintln!("correo calque: {f} read failed: {e} (skip)"),
                Ok(t) => report(&format!("{f}:"), &t),
            }
        }
    }
    if n == 0 {
        println!("CALQUE PASS: no english-verb + する/される splices");
        0
    } else if advisory {
        println!("CALQUE CANDIDATES: {n} (advisory)");
        0
    } else {
        println!("CALQUE FAIL: {n}");
        1
    }
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
    fn matches_dotfiles_hook_regression_cases() {
        // dotfiles hook の comment に記録された session-learned ケース（移植の回帰）。
        // BLOCK 側:
        for bad in [
            "これを citeする。",
            "flag された値を見る。", // 空白挟み・された
            "improveさせたい。",     // させたい
            "de-risk する方針。",    // ハイフン語＋空白
            "refactorさせる。",
        ] {
            assert!(!scan(bad).is_empty(), "block されなかった: {bad}");
        }
        // PASS 側: 大文字始まりの固有名・助詞挟み・漢語。
        for ok in [
            "GitHubした。", // 大文字始まり＝固有名
            "commit を実行する。",
            "解消されている。",
            "設計した。",
        ] {
            assert!(scan(ok).is_empty(), "誤検出した: {ok}");
        }
    }

    #[test]
    fn mentions_in_code_are_not_flagged() {
        assert!(scan("`deployする` は検出例。\n\n```\ndeployする\n```").is_empty());
    }
}
