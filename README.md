# correo

日本語の LLM slop を検出する lint。ここでの **slop ＝言語の不自然さ**（機械が混ぜた非母語的な日本語）で、**事実性・冗長性・hype は対象外**。

3 つの検出器:

- **codemix** — 地の文の latin/100字 密度（ルー語）。識別子・ALLCAPS 略語・allow-list 登録語は除外。
- **coinage** — Sudachi 形態素解析で辞書外の複合語（不自然な造語）を検出。
- **calque**（順次）— 英語動詞を「する」に接ぐ code-switching。

locate 層であって judge ではない ── 候補を flag するだけで、domain か gratuitous かの最終判定は呼び出し側（LLM-judge か人）が下す。

## install

```sh
brew install fuyutarow/tap/correo
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

# allow-list（除外語彙）を注入（統制語彙の .md を渡す）
correo codemix --exempt vocab.md docs/*.md
```

`--no-default-features` でビルドすると coinage を外した純 codemix になる（Sudachi 依存なし）。

## 出力

`codemix` は密度が閾値を超えた段落を `⚑ L{行}: N latin/100字` で列挙（常に exit 0・advisory）。`coinage --strict` は辞書外複合を候補列挙し、`--advisory` 無しなら違反時 exit 1。

## license

MIT OR Apache-2.0。同梱する SudachiDict は Apache-2.0（`dict/LICENSE-SudachiDict`）。
