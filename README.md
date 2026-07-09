# correo

日本語実用文の**機械床** — LLM slop（言語の不自然さ）と、木下是雄『理科系の作文技術』の機械判定可能な層を検査する lint。**事実性・hype は対象外**。

検出器は二軸:

**slop 軸**（機械が混ぜた非母語的な日本語 — locate 層）

- **codemix** — 地の文の latin/100字 密度（ルー語）。識別子・ALLCAPS 略語・allow-list 登録語は除外。
- **coinage** — Sudachi 形態素解析で辞書外の複合語（不自然な造語）を検出。
- **calque**（順次）— 英語動詞を「する」に接ぐ code-switching。

**木下軸**（機械が言い切れる床 — HARD）

- **kinoshita** — 一文の長さ・読点過多・ですます/である混在・慣用二重否定・ぼかし連発・指示語連鎖。

slop 軸は locate 層であって judge ではない ── 候補を flag するだけで、domain か gratuitous かの最終判定は呼び出し側（LLM-judge か人）が下す。kinoshita 軸だけは機械判定が最終（blocking 可）。

## roadmap — 木下被覆の層別

- **Tier 1（HARD）** — 実装済み＝上記 kinoshita。
- **Tier 2（MIX・proxy）** — 逆茂木の proxy（文頭の連体修飾チェーン長）→ flag のみ、judge が確認。
- **Tier 3（VIBE・座標のみ）** — トピックセンテンス・事実と意見・スリカエ → 段落第一文等の座標を構造化出力して LLM-judge に渡す（correo は判定しない）。
- 係り受けが要る原則（主述近接・修飾語順）は形態素の外 — proxy 化できた分だけ Tier 2 へ。

## install

formula は correo repo 内（`Formula/correo.rb`）が正本。repo 名が `homebrew-*` でないため tap は URL 明示形で足す:

```sh
brew tap fuyutarow/correo https://github.com/fuyutarow/correo.git
brew install fuyutarow/correo/correo
```

coinage 用の Sudachi 辞書（system.dic ほか）は formula が同梱し、binary が exe 相対で解決するため **out-of-box** で動く。

cargo から入れる場合（辞書は別途 `$CORREO_DICT_DIR` か `--dict-dir` で指定）:

```sh
cargo install --git https://github.com/fuyutarow/correo
```

## 使い方

```sh
# ルー語密度（file 無し = stdin）
echo '本文に framework や pipeline を混ぜた段落。' | correo codemix --threshold 8

# 造語（辞書外複合）
echo '構造腕を書く。' | correo coinage --strict --advisory

# allow-list（除外語彙）を注入（統制語彙の .md を渡す・複数可・coinage と同名 flag）
correo codemix --allow vocab.md docs/*.md

# 木下 HARD 床（既定: 一文 100 字・読点 4。--advisory で報告のみ＝exit 0）
correo kinoshita report.md

# 全検出器を 1 コマンドで。blocking は kinoshita のみ（codemix=advisory・
# coinage=strict 候補の advisory 報告・辞書が無ければ skip 明示）
correo check --allow vocab.md docs/*.md
```

`--no-default-features` でビルドすると coinage を外した純 codemix になる（Sudachi 依存なし）。

## 出力

`codemix` は密度が閾値を超えた段落を `⚑ L{行}: N latin/100字` で列挙（常に exit 0・advisory）。`coinage --strict` は辞書外複合を候補列挙し、`--advisory` 無しなら違反時 exit 1。

**codemix の判定床**: JA 40字未満の段落は判定しない（密度が統計的に無意味なため）。ただし**連続する箇条書きは 1 行群に集計**して床を越えさせる（LLM slop は短い bullet に湧くため。空行区切りの bullet 列も 1 単位）。code fence・inline code・URL・表・見出し行は地の文から除外して数える。

## license

MIT OR Apache-2.0。同梱する SudachiDict は Apache-2.0（`dict/LICENSE-SudachiDict`）。
