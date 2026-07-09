// coinage.rs — 造語検出 engine（Sudachi 見出し語 membership + 辞書 POS 所有の strict）。
// 判定原理・免除規則・残 FP class は run_coinage 直上の comment（本 file 内）が正本。
// feature "coinage"（外部 git = sudachi.rs pin）が無い build では stub が案内を出す。
// 回帰固定は本 file 末尾の #[cfg(test)]（cargo test --features coinage・要 Sudachi 辞書）。
use anyhow::Result;
use std::path::PathBuf;

/// clap の Coinage variant と 1:1 の名前つき引数（positional bool の羅列を避ける）。
/// stub build（--features coinage なし）では読まれない＝dead_code は cfg 差で正当。
#[cfg_attr(not(feature = "coinage"), allow(dead_code))]
pub struct CoinageArgs {
    pub dict_dir: Option<PathBuf>,
    /// allow-list registry の file path 群（.md の表 / 素の語リスト）。
    pub allow: Vec<String>,
    /// 語を直接注入する経路（correo.toml の allow — file を介さない）。
    pub allow_words: Vec<String>,
    pub diff: bool,
    pub strict: bool,
    pub advisory: bool,
    pub files: Vec<String>,
}

#[cfg(not(feature = "coinage"))]
pub fn run_coinage(_args: CoinageArgs) -> Result<i32> {
    eprintln!(
        "correo coinage: この binary は --features coinage なしで build されている。\n  有効化: cargo build --features coinage（既定の default feature を有効のまま build）"
    );
    Ok(2)
}

#[cfg(feature = "coinage")]
use anyhow::Context;
#[cfg(feature = "coinage")]
use regex::Regex;
#[cfg(feature = "coinage")]
use std::fs;
#[cfg(feature = "coinage")]
use sudachi::analysis::Mode;
#[cfg(feature = "coinage")]
use sudachi::analysis::Tokenize;
#[cfg(feature = "coinage")]
use sudachi::analysis::stateless_tokenizer::StatelessTokenizer;
#[cfg(feature = "coinage")]
use sudachi::config::Config;
#[cfg(feature = "coinage")]
use sudachi::dic::dictionary::JapaneseDictionary;

/// 複合成分＝（表層形, in-context 品詞 [品詞1,品詞2,品詞3]）。
#[cfg(feature = "coinage")]
type Component = (String, [String; 3]);

// strict(HARD) 判定の知識は辞書（UniDic 品詞体系）が所有する — 手書き stoplist を持たない
// （旧 AFFIX_STOP 列挙は 的（接尾辞）と数詞の欠落で 数理的/一例/三軸 を実測誤爆した＝列挙の
//   非汎化・2026-07-06 廃止）。二つの辞書信号:
//   (1) in-context POS 免除 = 接頭辞・接尾辞（辞書が接辞と宣言）・名詞-数詞（数量表現: 一例・三軸）・
//       名詞-普通名詞-{サ変可能/形状詞可能 等}（機能性下位分類: 値・可）。
//   (2) lexicon 級 接尾辞語義 免除 = in-context で 一般 に潰れても、lexicon に 接尾辞（助数詞を除く）
//       語義を持つ字（法/木/正 等）は語形成要素（測定法・推定法 を救う）。助数詞のみの字（床/幅）は
//       数え口であって語形成でないため免除しない（機械床 は落ちたまま）。
//   違反 = 単漢字（漢字/々）成分が両信号とも独立名詞（名詞-普通名詞-一般 かつ 接尾辞語義なし）
//          ＝独立名詞を無理に複合させた形（床/腕/核 の具象比喩ラベル class はここに落ちる）。
//   数詞を含む複合は数量表現として全体免除 — 既知の coined 数詞複合（二腕/二肢）と接尾辞形
//   （構造肢: 肢=接尾辞）は corpus 上「自然な形」ゆえ strict を構造的に通る＝確定済み造語の
//   決定的禁止は prh-banned residue（墓場）が担う（分業）。
//   残 FP class（正直に）: 例/図/層/軸 tail（使用例・物理層 等）は辞書が接尾辞語義を持たず flag
//   され得る — channel は allow-list 登録・深い治療は n-gram corpus か judge 層（advisory）。

/// corpus 語彙表（jawiki 記事タイトル・BCCWJ 長単位語彙表等）を読む。
/// 1 行 1 語・TSV は先頭列・#/空行は無視。用途は check の strict 候補照合 —
/// 「辞書に無い」だけでは造語と実在語（物理層）を区別できないため、実コーパスの
/// 出現実績を第二の証拠にする。
#[cfg(feature = "coinage")]
pub fn load_corpus(path: &Path) -> Result<HashSet<String>> {
    let t =
        fs::read_to_string(path).with_context(|| format!("corpus 読込失敗: {}", path.display()))?;
    Ok(t.lines()
        .map(|l| l.split('\t').next().unwrap_or("").trim())
        .filter(|s| !s.is_empty() && !s.starts_with('#'))
        .map(str::to_string)
        .collect())
}

/// latin 語 token（ASCII 英数と -_ のみ・英字を含む）。混種語 run の latin 側成分。
#[cfg(feature = "coinage")]
fn is_latin_word(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        && s.chars().any(|c| c.is_ascii_alphabetic())
}

#[cfg(feature = "coinage")]
fn is_kanji_kata(c: char) -> bool {
    ('\u{4E00}'..='\u{9FFF}').contains(&c)      // CJK 統合漢字
        || ('\u{30A0}'..='\u{30FF}').contains(&c) // カタカナ
        || c == '\u{3005}' // 々
}

#[cfg(feature = "coinage")]
use std::collections::HashSet;
#[cfg(feature = "coinage")]
use std::io::Read;
#[cfg(feature = "coinage")]
use std::path::Path;

#[cfg(feature = "coinage")]
struct Coinage {
    dict: JapaneseDictionary,
    allow: HashSet<String>,
}

#[cfg(feature = "coinage")]
impl Coinage {
    fn new(dict_dir: &Path, allow_files: &[String]) -> Result<Self> {
        let config = Config::new(
            Some(dict_dir.join("sudachi.json")),
            Some(dict_dir.to_path_buf()),
            Some(dict_dir.join("system.dic")),
        )
        .map_err(|e| anyhow::anyhow!("Sudachi config 失敗: {e}"))?;
        let dict = JapaneseDictionary::from_cfg(&config)
            .map_err(|e| anyhow::anyhow!("辞書 load 失敗: {e}"))?;
        let mut allow = HashSet::new();
        // allow-list registry（.md）を allow 源に: | `term …` | 行の backtick セル。
        // セル全体と先頭 token を語として登録（allow-list registry の単一 channel）。
        let cell_rx = Regex::new(r"^\|\s*`([^`]+)`").unwrap();
        for f in allow_files {
            let t = fs::read_to_string(f).with_context(|| format!("allowlist 読込失敗 {f}"))?;
            if f.ends_with(".md") {
                for line in t.lines() {
                    if let Some(c) = cell_rx.captures(line) {
                        let cell = c[1].trim().to_string();
                        if let Some(first) = cell.split_whitespace().next() {
                            allow.insert(first.to_string());
                        }
                        allow.insert(cell);
                    }
                }
            } else {
                for line in t.lines() {
                    let s = line.trim();
                    if !s.is_empty() && !s.starts_with('#') {
                        allow.insert(s.to_string());
                    }
                }
            }
        }
        Ok(Coinage { dict, allow })
    }

    /// 1 行を Mode C で解析し、辞書に長単位化されない内容語複合を（成分品詞つきで）返す。
    /// 混種語対応（2026-07-09 dogfood: 「slop 軸」「機械床」class が codemix の密度床と
    /// coinage の和字限定 run の隙間に落ちて不可視だった）: latin token も run に参加させ、
    /// latin→和字 の境界に限り単一空白で複合を橋渡しする（「slop軸」「slop 軸」の両表記を捕捉）。
    /// pure-latin run（英語の語列）は複合でないので捨てる — 和字を含む run だけが候補。
    fn scan_line(&self, line: &str) -> Vec<(String, Vec<Component>)> {
        let tok = StatelessTokenizer::new(&self.dict);
        let mut out = Vec::new();
        let morphs = match tok.tokenize(line, Mode::C, false) {
            Ok(m) => m,
            Err(_) => return out, // 解析不能行は判定対象外 (バイナリ断片等)
        };
        let mut run: Vec<Component> = Vec::new();
        let mut bridged = false; // 直前 token が「複合を繋ぐ空白」だったか（連続空白は繋がない）
        let flush = |run: &mut Vec<Component>, out: &mut Vec<(String, Vec<Component>)>| {
            let has_native = run.iter().any(|(s, _)| s.chars().any(is_kanji_kata));
            if run.len() >= 2 && has_native {
                let compound: String = run.iter().map(|(s, _)| s.as_str()).collect();
                if !self.allow.contains(&compound) {
                    out.push((compound, run.clone()));
                }
            }
            run.clear();
        };
        for m in morphs.iter() {
            let pos = m.part_of_speech();
            let get = |i: usize| pos.get(i).map(|s| s.to_string()).unwrap_or_default();
            let pos3 = [get(0), get(1), get(2)];
            let surf = m.surface().to_string();
            let is_native = matches!(pos3[0].as_str(), "名詞" | "接頭辞" | "接尾辞")
                && surf.chars().any(is_kanji_kata);
            let content = is_native || is_latin_word(&surf);
            if content {
                run.push((surf, pos3));
                bridged = false;
            } else if !bridged
                && surf.trim().is_empty()
                && run.last().is_some_and(|(s, _)| is_latin_word(s))
            {
                // latin の直後の単一空白は保留 — 次が content なら複合を繋ぐ（"slop 軸"）。
                // 和字→空白は繋がない（「評価 結果」を複合にしない）。
                bridged = true;
            } else {
                flush(&mut run, &mut out);
                bridged = false;
            }
        }
        flush(&mut run, &mut out);
        out
    }

    /// 単漢字 surface が lexicon に語形成の接尾辞語義（接尾辞かつ 助数詞でない）を持つか。
    /// in-context では 普通名詞-一般 に潰れる 法/木/正 等を、辞書自身の語義で機能クラスへ救う。
    fn has_suffix_reading(&self, surf: &str) -> bool {
        let bytes = surf.as_bytes();
        for e in self.dict.lexicon().lookup(bytes, 0) {
            if e.end != bytes.len() {
                continue;
            }
            if let Ok(wi) = self.dict.lexicon().get_word_info(e.word_id) {
                let pos = self.dict.grammar().pos_components(wi.pos_id());
                if pos.first().map(|s| s.as_str()) == Some("接尾辞")
                    && pos.get(2).map(|s| s.as_str()) != Some("助数詞")
                {
                    return true;
                }
            }
        }
        false
    }

    /// strict(HARD) 判定（知識は辞書所有・冒頭 comment の 2 信号）。
    /// 接尾辞語義の救済は**非語頭に限る**（2026-07-09 dogfood: 家 は接尾辞語義（政治家）を
    /// 持つが語頭では語形成しない — 位置を見ないと「家造語」class の頭付け造語が免除される）。
    fn is_strict_hit(&self, components: &[Component]) -> bool {
        if components.iter().any(|(_, pos)| pos[1] == "数詞") {
            return false;
        }
        components.iter().enumerate().any(|(i, (surf, pos))| {
            let single_kanji = {
                let mut cs = surf.chars();
                match (cs.next(), cs.next()) {
                    (Some(ch), None) => ('\u{4E00}'..='\u{9FFF}').contains(&ch) || ch == '\u{3005}',
                    _ => false,
                }
            };
            single_kanji
                && pos[0] == "名詞"
                && pos[1] == "普通名詞"
                && pos[2] == "一般"
                && !(i > 0 && self.has_suffix_reading(surf))
        })
    }
}

/// 走査結果 1 件（data 層 — 印字と分離。JSON 出力=Tier 3 judge 界面の第二消費者が要求・2026-07-09）。
#[cfg(feature = "coinage")]
pub struct CoinageHit {
    pub file: String,
    pub line: usize,
    pub compound: String,
    pub components: Vec<String>,
}

/// 3 入力モード（diff / stdin / files）共通の走査。strict filter 適用済みの hit と
/// allow-list 語数を返す。印字はしない（run_coinage と check --format json が消費する）。
#[cfg(feature = "coinage")]
pub fn collect(args: &CoinageArgs) -> Result<(Vec<CoinageHit>, usize)> {
    let dict_dir = resolve_dict_dir(args.dict_dir.clone())
        .context("--dict-dir か CORREO_DICT_DIR か $HOME/.cache/correo が必要")?;
    let mut engine = Coinage::new(&dict_dir, &args.allow)
        .context("engine 初期化失敗（--dict-dir か $CORREO_DICT_DIR で Sudachi 辞書を指定）")?;
    engine.allow.extend(args.allow_words.iter().cloned());

    let mut hits: Vec<CoinageHit> = Vec::new();
    let scan = |fname: &str, lineno: usize, line: &str, hits: &mut Vec<CoinageHit>| {
        for (compound, parts) in engine.scan_line(line) {
            if args.strict && !engine.is_strict_hit(&parts) {
                continue;
            }
            hits.push(CoinageHit {
                file: fname.to_string(),
                line: lineno,
                compound,
                components: parts.iter().map(|(s, _)| s.clone()).collect(),
            });
        }
    };

    if args.diff {
        // stdin の unified diff (git diff -U0) から追加行のみ判定
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf).ok();
        let mut cur_file = String::from("?");
        let mut lineno: usize = 0;
        let hunk_re = Regex::new(r"^@@ -\S+ \+(\d+)").unwrap();
        for line in buf.lines() {
            if let Some(p) = line.strip_prefix("+++ b/") {
                cur_file = p.to_string();
            } else if let Some(c) = hunk_re.captures(line) {
                lineno = c[1].parse::<usize>().unwrap_or(1);
            } else if let Some(added) = line.strip_prefix('+')
                && !line.starts_with("+++")
            {
                scan(&cur_file, lineno, added, &mut hits);
                lineno += 1;
            }
            // -U0 では context 行は無い; 削除行 (-) は新 file 行番号を進めない
        }
    } else if args.files.is_empty() {
        // fence 内はコード例・inline code は語の「言及」＝どちらも散文でない（fence strip は
        // 同数改行置換で行番号不変・prose.rs 共有。inline strip は行内処理で行番号に影響なし）。
        // diff モードは対象外: 断片に fence ペアの文脈が無く、対にならない ``` を誤爆させるより
        // 追加行をそのまま見る方が安全。
        let inline = Regex::new(r"`[^`]*`").unwrap();
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf).ok();
        let buf = crate::prose::strip_fences(&buf);
        for (i, line) in buf.lines().enumerate() {
            // markdown 表は構造データで散文でない — cell の圧縮表記（読点数・文連続 等）を
            // 造語候補にしない（README 規則台帳で実測した FP class・2026-07-09）。
            if line.trim_start().starts_with('|') {
                continue;
            }
            scan("-", i + 1, &inline.replace_all(line, ""), &mut hits);
        }
    } else {
        let inline = Regex::new(r"`[^`]*`").unwrap();
        for f in &args.files {
            match fs::read_to_string(f) {
                Err(e) => eprintln!("correo coinage: {f} 読込失敗: {e} (skip)"),
                Ok(t) => {
                    let t = crate::prose::strip_fences(&t);
                    for (i, line) in t.lines().enumerate() {
                        if line.trim_start().starts_with('|') {
                            continue; // 表は散文でない（stdin 側と同じ規約）
                        }
                        scan(f, i + 1, &inline.replace_all(line, ""), &mut hits);
                    }
                }
            }
        }
    }
    Ok((hits, engine.allow.len()))
}

#[cfg(feature = "coinage")]
pub fn run_coinage(args: CoinageArgs) -> Result<i32> {
    let advisory = args.advisory;
    let (hits, allow_len) = collect(&args)?;
    let violations: Vec<String> = hits
        .iter()
        .map(|h| {
            let surfs: Vec<&str> = h.components.iter().map(|s| s.as_str()).collect();
            format!(
                "{}:{}: 「{}」 is not a dictionary headword (components={surfs:?}) — rewrite in standard terms, or register in allow-list (--allow)",
                h.file, h.line, h.compound
            )
        })
        .collect();

    if violations.is_empty() {
        println!("COINAGE PASS: no out-of-dictionary compounds (allowlist {allow_len} entries)");
        Ok(0)
    } else if advisory {
        // locate 層（advisory）: 候補を報告し judge の 3-way 分類へ回す。blocking は
        // prh residue（確定造語）が担う — strict tier の実測 FP（語/層/例 tail 等の
        // 生産的接尾辞様に辞書が接尾辞語義を持たない精度天井・2026-07-06）による裁定。
        println!(
            "COINAGE CANDIDATES: {} (advisory — judge triages: natural / register in allow / confirmed coinage → deny)",
            violations.len()
        );
        for v in &violations {
            println!("  {v}");
        }
        Ok(0)
    } else {
        println!(
            "COINAGE FAIL: {} (criterion = Sudachi mode-C headword membership)",
            violations.len()
        );
        for v in &violations {
            println!("  {v}");
        }
        Ok(1)
    }
}

/// 辞書 dir の解決順: --dict-dir、次に $CORREO_DICT_DIR、次に exe 相対 ../share/correo/dict
/// （brew の bundle 辞書）、最後に ~/.cache/correo（mise run setup:sudachidict の配置先）。
/// 自動発見の 2 経路（exe 相対・cache）は system.dic 存在で filter — 空 dir を掴んで不明瞭な
/// engine エラーになるのを防ぐ（2026-07-09）。明示指定（--dict-dir/env）は検証せず通す:
/// ユーザ意図のある path は engine の実エラーを表面化させる方が診断に良い。
/// exe 相対を挟むことで、brew install 後は env 無設定でも同梱辞書を掴む（out-of-box）。
#[cfg(feature = "coinage")]
fn resolve_dict_dir(cli: Option<PathBuf>) -> Option<PathBuf> {
    let with_system_dic = |d: PathBuf| d.join("system.dic").is_file().then_some(d);
    cli.or_else(|| std::env::var("CORREO_DICT_DIR").ok().map(PathBuf::from))
        .or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent()?.parent().map(|d| d.join("share/correo/dict")))
                .and_then(with_system_dic)
        })
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join(".cache/correo"))
                .and_then(with_system_dic)
        })
}

// ── 回帰固定（旧 --self-test subcommand を Rust idiom へ移設・2026-07-06）──
// strict スペック（POS 所有・列挙なし）の受入 test。実行: cargo test --features coinage
// = cargo test --features coinage（要 Sudachi 辞書 = 辞書を用意）。
#[cfg(all(test, feature = "coinage"))]
mod tests {
    use super::*;
    use std::sync::OnceLock;

    #[test]
    fn suffix_reading_rescues_only_non_initial_position() {
        // 回帰 (2026-07-09 dogfood): linter の登録簿自身に書いてしまった「家造語」が strict を
        // すり抜けた。家 の接尾辞語義（政治家）は語尾でしか語形成しない — 語頭では救済しない。
        let e = engine();
        let hits = e.scan_line("家造語を登録する。");
        let hit = hits
            .iter()
            .find(|(c, _)| c == "家造語")
            .expect("家造語 が候補に出ず");
        assert!(
            e.is_strict_hit(&hit.1),
            "語頭の接尾辞語義が 家造語 を免除した"
        );
        // 測定法 は 法 が語尾＝接尾辞語義の本来の位置 — strict を通る（従来どおり）。
        let hits = e.scan_line("測定法を使う。");
        if let Some((_, parts)) = hits.iter().find(|(c, _)| c == "測定法") {
            assert!(!e.is_strict_hit(parts), "測定法 が strict に落ちた");
        }
    }

    #[test]
    fn hybrid_latin_kanji_compound_is_flagged_with_and_without_space() {
        // 回帰 (2026-07-09 dogfood): 「slop 軸」class の混種語が codemix 密度床と coinage の
        // 和字限定 run の隙間に落ちて不可視だった。両表記（密着/空白）とも strict で捕まえる。
        let e = engine();
        for line in ["このslop軸を使う。", "この slop 軸を使う。"] {
            let hits = e.scan_line(line);
            let hit = hits.iter().find(|(c, _)| c == "slop軸");
            assert!(
                hit.is_some(),
                "{line} で slop軸 が取れず: {:?}",
                hits.iter().map(|(c, _)| c).collect::<Vec<_>>()
            );
            assert!(
                e.is_strict_hit(&hit.unwrap().1),
                "slop軸 が strict を通った"
            );
        }
    }

    #[test]
    fn natural_hybrid_formations_pass_strict() {
        // 製 は辞書が接尾辞と知っている・出力 は複漢字語 — 混種でも自然な形は strict を通る
        // （Rust製/JSON出力 を誤爆させない・知識は辞書所有＝列挙しない）。
        let e = engine();
        for (line, comp) in [
            ("Rust 製の道具。", "Rust製"),
            ("JSON 出力を見る。", "JSON出力"),
        ] {
            let hits = e.scan_line(line);
            if let Some((_, parts)) = hits.iter().find(|(c, _)| c == comp) {
                assert!(
                    !e.is_strict_hit(parts),
                    "{comp} が strict に落ちた（自然な混種形）"
                );
            }
        }
    }

    #[test]
    fn collect_skips_code_fences_in_file_mode() {
        // 回帰 (2026-07-09 dogfood): README のコード例内の「構造腕」を造語 flag した実害。
        // fence 内はコード＝散文でない。行番号は同数改行置換で不変。
        use std::io::Write;
        let mut f = tempfile::NamedTempFile::new().expect("tmp");
        write!(f, "構造腕を書く。\n\n```\n構造腕の例\n```\n").unwrap();
        let args = CoinageArgs {
            dict_dir: None,
            allow: vec![],
            allow_words: vec![],
            diff: false,
            strict: true,
            advisory: true,
            files: vec![f.path().display().to_string()],
        };
        let (hits, _) = collect(&args).expect("Sudachi 辞書が必要（mise run setup:sudachidict）");
        let got: Vec<(String, usize)> = hits.iter().map(|h| (h.compound.clone(), h.line)).collect();
        assert_eq!(
            got,
            vec![("構造腕".to_string(), 1)],
            "fence 内が flag された: {got:?}"
        );
    }

    fn engine() -> &'static Coinage {
        static E: OnceLock<Coinage> = OnceLock::new();
        E.get_or_init(|| {
            let dir = resolve_dict_dir(None).expect("HOME 未設定");
            Coinage::new(&dir, &[]).expect("Sudachi 辞書が必要（辞書を用意）")
        })
    }

    fn strict_hits(text: &str) -> Vec<String> {
        let e = engine();
        e.scan_line(text)
            .into_iter()
            .filter(|(_, p)| e.is_strict_hit(p))
            .map(|(c, _)| c)
            .collect()
    }

    /// 比喩ラベル class（単漢字 名詞-普通名詞-一般 成分・数詞なし）を検出する。
    /// 値列 = 列（一般）ゆえ flag 側（2026-07-06 spec 変更・家内略記は allow-list 登録で通す）。
    #[test]
    fn flags_metaphor_label_compounds() {
        for t in ["構造腕", "機械床", "値列"] {
            let hits = strict_hits(&format!("{t} を書く。"));
            assert!(hits.contains(&t.to_string()), "見逃し: {t} (hits={hits:?})");
        }
    }

    /// 見出し語・数詞複合・接尾辞複合は通す。一例/数理的/三軸 = 旧 AFFIX_STOP 列挙の
    /// 実測誤爆（実測）— 再発防止で固定。
    #[test]
    fn passes_headwords_and_functional_compounds() {
        for t in [
            "機械学習",
            "深層学習",
            "計算機科学",
            "一例",
            "数理的",
            "三軸",
        ] {
            let hits = strict_hits(&format!("{t} を書く。"));
            assert!(hits.is_empty(), "誤検出: {t} (hits={hits:?})");
        }
    }

    /// 免除は辞書 POS が所有: 接尾辞（済/外）・形状詞可能（可）・接頭辞（実）・
    /// lexicon 接尾辞語義（法 — 測定法/推定法 を救う）。
    #[test]
    fn functional_class_calibration() {
        for s in [
            "測定設計 を行う。",
            "配線済 と 使用可 と フレーム外 を確認する。",
            "実コヒーレンス を測る。",
            "測定法 と 推定法 を比べる。",
        ] {
            let hits = strict_hits(s);
            assert!(hits.is_empty(), "strict 過剰: {s} (hits={hits:?})");
        }
    }

    /// residue 分界: 構造肢（肢=接尾辞）・二腕（数詞複合）は corpus 上自然な形ゆえ
    /// strict を通す — 確定済み coined ラベルの決定的禁止は prh-banned residue が担う。
    #[test]
    fn residue_boundary_is_prh_territory() {
        for s in ["構造肢 を数える。", "二腕 で測る。"] {
            let hits = strict_hits(s);
            assert!(hits.is_empty(), "residue 分界破れ: {s} (hits={hits:?})");
        }
    }

    /// allow 源 = 統制語彙 registry（.md の | `term …` | 行）を parse できる。
    #[test]
    fn allow_accepts_md_registry() {
        let f = tempfile::NamedTempFile::with_suffix(".md").expect("tempfile");
        std::fs::write(f.path(), "| `接触集合 Z (contact set)` | home | g |\n").unwrap();
        let dir = resolve_dict_dir(None).expect("HOME 未設定");
        let e = Coinage::new(&dir, &[f.path().to_string_lossy().to_string()]).unwrap();
        assert!(e.allow.contains("接触集合"));
        assert!(e.allow.contains("接触集合 Z (contact set)"));
    }
}
