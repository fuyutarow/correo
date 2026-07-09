// config.rs — correo.toml の自動発見（biome.json 方式: 引数ゼロで設定が効く）。
// cwd から根へ辿って最初の correo.toml を読む。無ければ既定値 — 設定ファイルは必須にしない。
// 優先順位は CLI flag > correo.toml > 組み込み既定（解決は main 側）。
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
    #[serde(default)]
    pub codemix: CodemixCfg,
    #[serde(default)]
    pub kinoshita: KinoshitaCfg,
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
[codemix]
threshold = 6.0
[kinoshita]
max-sentence = 90
"#,
        )
        .expect("parse");
        assert_eq!(cfg.allow.len(), 2);
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
