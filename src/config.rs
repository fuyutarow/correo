// config.rs — correo.toml の自動発見。
// Biome から借りたのは【機構】（自動発見・引数ゼロで設定が効く・CLI flag > 設定 > 既定の優先順位）
// であって file 形式ではない — Biome は biome.json/biome.jsonc（JSONC・自前 parser）、correo は
// Rust 圏の母語である TOML（Cargo.toml/rustfmt.toml/mise.toml と同じ）。コメントが言語仕様に
// あるため、allow の「裁定理由をコメントで残す」運用が標準機能で成立する（JSONC はこれを
// 得るために Biome が parser を自作した）。typo 安全は deny_unknown_fields が担い、エディタ
// 補完は taplo/tombi 向け JSON Schema の配布で将来対応（roadmap）。
// cwd から根へ辿って最初の correo.toml を読む。無ければ既定値 — 設定ファイルは必須にしない。
// allow は「judge の裁定を経た語」の登録先 — 一回きりの言及は inline 抑制（suppress.rs）で
// 逃がし、registry を太らせない。
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// 裁定済みの許容語彙（codemix の exempt と coinage の allow の共通源）。
    #[serde(default)]
    pub allow: Vec<String>,
    /// 確定した造語の禁止辞書（語 → 書き直しの案）。HARD＝exit 1 に数える。
    /// judge の 3-way 裁定の第三バケツ — 自然→無視 / 使い続ける→allow / 確定造語→ここ。
    #[serde(default)]
    pub deny: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub codemix: CodemixCfg,
    #[serde(default)]
    pub kinoshita: KinoshitaCfg,
    #[serde(default)]
    pub coinage: CoinageCfg,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct CoinageCfg {
    /// 実在語彙表（1 行 1 語・TSV は先頭列・# 行は無視。setup:corpus = SudachiDict full lex ∪
    /// jawiki タイトル）。役割は「実在の証明で候補を**消す**」だけ（物理層・混種語 等を救い
    /// advisory ノイズと allow 保守を減らす）。不在を error に昇格させることは**しない** —
    /// 使用例/実用文 級の生産的複合は有限語彙表に載らない（387万語で実測 0・2026-07-09）ため、
    /// 不在≠造語。造語の確定は judge の裁定＝[deny] が持つ。
    pub corpus: Option<PathBuf>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct CodemixCfg {
    pub threshold: Option<f64>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct KinoshitaCfg {
    pub max_sentence: Option<usize>,
    pub max_ten: Option<usize>,
}

/// cwd から根へ辿り最初の correo.toml を読む。パース失敗は握り潰さず Err（設定の黙殺は事故）。
pub fn discover() -> anyhow::Result<(Option<PathBuf>, Config)> {
    let mut dir = std::env::current_dir()?;
    loop {
        let p = dir.join("correo.toml");
        if p.is_file() {
            let s = std::fs::read_to_string(&p)?;
            let cfg = toml::from_str(&s)
                .map_err(|e| anyhow::anyhow!("{} の parse に失敗: {e}", p.display()))?;
            return Ok((Some(p), cfg));
        }
        if !dir.pop() {
            return Ok((None, Config::default()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_documented_shape() {
        let cfg: Config = toml::from_str(
            r#"
allow = ["ルー語", "slop"]
[deny]
"機械床" = "標準的な言い方へ書き直す"
[codemix]
threshold = 6.0
[kinoshita]
max-sentence = 90
"#,
        )
        .expect("parse");
        assert_eq!(cfg.allow.len(), 2);
        assert_eq!(cfg.deny.len(), 1);
        assert_eq!(cfg.codemix.threshold, Some(6.0));
        assert_eq!(cfg.kinoshita.max_sentence, Some(90));
        assert_eq!(cfg.kinoshita.max_ten, None);
    }

    #[test]
    fn unknown_keys_are_rejected_not_silently_ignored() {
        // typo した設定が黙って無視される事故を防ぐ（deny_unknown_fields）。
        assert!(toml::from_str::<Config>("thresold = 8").is_err());
    }
}
