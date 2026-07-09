// density.rs — 情報密度と近重複の検出（著者不問の品質信号・2026-07-09）。
// 「AI が書いたか」を確率で当てる路線（GLTR/DetectGPT 系）はイタチごっこに負けた系譜なので
// 採らない。代わりに「薄い・重複」という品質欠陥そのものを決定的に測る — 人間の手抜きも
// 同時に捕まるのは仕様。核は二つ:
//   (1) low-information-density — 段落の deflate 圧縮率（bits/字）。反復・定型は圧縮が効きすぎる。
//       LM 不要（"gzip beats BERT", Jiang+ 2023 の系譜）。実測較正（2026-07-09）:
//       正常な技術文 18.6 / 反復 slop 10.1 / 定型 slop 11.3 → 閾値 14.0・60 字床
//       （短文は zlib header の overhead 支配で bits/字 が跳ねる）。
//   (2) near-duplicate — 段落間の文字 bigram cosine。はじめに≒おわりに・コピペ水増しを
//       埋め込みモデル無しで捕まえる。実測: 言い換え水増し 0.80 / 無関係 0.00 → 閾値 0.65。
// どちらも advisory（MIX）— 詩的反復・意図した再掲があり得るので最終確認は judge/人。
use std::collections::HashMap;
use std::io::Write;

use crate::prose::prose_units;

const MIN_CHARS: usize = 60; // 圧縮率用（短文は zlib overhead 支配）
// 実測の再較正（2026-07-09 dogfood）: 初期閾値 14.0 は玩具サンプル（純和文の密な一段落 18.6）
// への過適合で、良い README の段落が術語の正当な反復で 11.8–13.7 に沈み 11 件誤爆した。
// 安全に分離できるのは極端な反復（反復 slop 実測 10.1）だけ — 保守的に 11.0 で運用し、
// 定型 slop（実測 11.3）は取り逃す。これは仕様: 誤爆が信頼を殺すので、境界域は judge の領分。
const MIN_BITS_PER_CHAR: f64 = 11.0;
const DUP_MIN_CHARS: usize = 40; // 近重複用（はじめに/おわりに は 50–80 字が現実的）
const DUP_COSINE: f64 = 0.65;

pub struct Finding {
    pub line: usize,
    pub rule: &'static str,
    pub msg: String,
}

pub fn scan(text: &str) -> Vec<Finding> {
    let units = prose_units(text);
    let mut out = Vec::new();

    // (1) 圧縮率 — 単位は散文段落（bullet 行群含む・fence/表は prose.rs が除外済み）
    let sized: Vec<(usize, &str, usize)> = units
        .iter()
        .map(|u| (u.line, u.text.as_str(), u.text.chars().count()))
        .collect();
    for (line, t, chars) in &sized {
        if *chars < MIN_CHARS {
            continue;
        }
        // 較正の適用範囲: 和文優勢の段落のみ。bits/字 は文字数で割るが zlib はバイトを
        // 圧縮するため、ASCII（1 byte/字）混在が多いと構造的に低く出て誤爆する
        // （2026-07-09 dogfood: latin 混じりの README 11 段落が 11.8–13.7 で誤爆した実測）。
        // script 非依存にする精密化は「同一段落の文字シャッフル比較」だが、まず較正が
        // 成立した領域だけで判定する。
        let ascii = t.chars().filter(|c| c.is_ascii()).count();
        if ascii as f64 / *chars as f64 > 0.15 {
            continue;
        }
        let bpc = deflate_bits(t) / *chars as f64;
        if bpc < MIN_BITS_PER_CHAR {
            out.push(Finding {
                line: *line,
                rule: "low-information-density",
                msg: format!(
                    "情報密度が低い（{bpc:.1} bits/字 < {MIN_BITS_PER_CHAR}）— 反復・定型の疑い。中身を足すか削る"
                ),
            });
        }
    }

    // (2) 近重複 — 全段落の総当たり（文書内の段落数なら O(P^2) で十分軽い）
    let vecs: Vec<Option<HashMap<(char, char), f64>>> = sized
        .iter()
        .map(|(_, t, chars)| (*chars >= DUP_MIN_CHARS).then(|| bigram_counts(t)))
        .collect();
    for i in 0..sized.len() {
        for j in (i + 1)..sized.len() {
            let (Some(a), Some(b)) = (&vecs[i], &vecs[j]) else {
                continue;
            };
            let c = cosine(a, b);
            if c >= DUP_COSINE {
                out.push(Finding {
                    line: sized[j].0,
                    rule: "near-duplicate",
                    msg: format!(
                        "L{} とほぼ同内容（類似 {c:.2}）— 水増しか貼り直し。片方へ集約する",
                        sized[i].0
                    ),
                });
            }
        }
    }
    out.sort_by_key(|f| f.line);
    out
}

fn deflate_bits(s: &str) -> f64 {
    let mut enc = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
    enc.write_all(s.as_bytes()).expect("in-memory deflate");
    let n = enc.finish().expect("in-memory deflate").len();
    (n * 8) as f64
}

fn bigram_counts(s: &str) -> HashMap<(char, char), f64> {
    let cs: Vec<char> = s.chars().filter(|c| !c.is_whitespace()).collect();
    let mut m = HashMap::new();
    for w in cs.windows(2) {
        *m.entry((w[0], w[1])).or_insert(0.0) += 1.0;
    }
    m
}

fn cosine(a: &HashMap<(char, char), f64>, b: &HashMap<(char, char), f64>) -> f64 {
    let dot: f64 = a
        .iter()
        .map(|(k, va)| va * b.get(k).copied().unwrap_or(0.0))
        .sum();
    let na: f64 = a.values().map(|v| v * v).sum::<f64>().sqrt();
    let nb: f64 = b.values().map(|v| v * v).sum::<f64>().sqrt();
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na * nb)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repetitive_paragraph_is_low_density_and_normal_prose_is_not() {
        // 較正実測（2026-07-09）と同サンプル系: 反復 slop ~10 bits/字 vs 正常 ~18.6。
        // 境界サンプル（bpc≈10–11）は encoder 差（zlib/miniz_oxide）で閾値を跨ぐ — 検証は
        // 明確に極端な反復で行う（閾値からの余裕を持たせる・2026-07-09 の較正教訓）。
        let slop = "本製品は素晴らしい体験を提供します。素晴らしい体験は素晴らしい価値を生みます。素晴らしい価値は素晴らしい未来を作ります。素晴らしい未来は素晴らしい体験から始まります。素晴らしい体験は素晴らしい価値を生みます。素晴らしい価値は素晴らしい未来を作ります。素晴らしい未来は素晴らしい体験から始まります。";
        let v = scan(slop);
        assert!(
            v.iter().any(|f| f.rule == "low-information-density"),
            "反復 slop が低密度と判定されず"
        );
        let normal = "本手法は測定系の較正誤差を三段階で分離する。第一段階では基準信号との相互相関から時間遅れを推定し、第二段階で振幅の非線形性を多項式回帰で補正する。";
        assert!(
            scan(normal)
                .iter()
                .all(|f| f.rule != "low-information-density"),
            "正常な技術文が低密度と誤判定"
        );
        // 短文は判定しない（overhead 支配）。
        assert!(scan("測定は三回行った。結果を表1に示す。").is_empty());
        // latin 混在段落は判定しない — ASCII は 1 byte/字 で bits/字 が構造的に下がり、
        // 純和文で取った較正が適用できない（dogfood 実測の FP class）。
        let mixed = "correo の check は codemix と coinage と readability を advisory と error の二値で報告し、correo の check は judge へ構造化した finding を渡す。correo の check は exit code で CI を止める。";
        assert!(
            scan(mixed)
                .iter()
                .all(|f| f.rule != "low-information-density"),
            "latin 混在段落を誤爆"
        );
    }

    #[test]
    fn paraphrased_duplicate_paragraphs_are_flagged_but_unrelated_are_not() {
        let dup = "本稿では測定系の較正誤差を分離する手法を提案する。三段階の分離により残差を白色雑音水準まで低減できることを示す。\n\n本稿では測定系の較正誤差を分離する手法を提案した。三段階の分離により残差が白色雑音水準まで低減されることを示した。";
        let v = scan(dup);
        let hit = v.iter().find(|f| f.rule == "near-duplicate");
        assert!(hit.is_some(), "言い換え水増しが検出されず");
        assert_eq!(hit.unwrap().line, 3, "重複の後段の行を指すべき");
        let ok = "本稿では測定系の較正誤差を分離する手法を提案する。三段階の分離により残差を白色雑音水準まで低減できることを示す。\n\n実験装置は恒温槽内に設置し、温度変動を一定に保った。試料は毎回新品を用いて摩耗の影響を除いた。";
        assert!(
            scan(ok).iter().all(|f| f.rule != "near-duplicate"),
            "無関係な段落を重複と誤判定"
        );
    }
}
