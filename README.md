# correo

日本語実用文の lint — LLM slop（言語の不自然さ）と、木下是雄『理科系の作文技術』の原則のうち**機械的に判定できるもの**を検査する。**事実性・hype は対象外**。

検出器は二系統:

**LLM slop の検出**（機械が混ぜた非母語的な日本語。候補を挙げるだけで判定しない）

- **codemix** — 地の文の latin/100字 密度（ルー語）。識別子・ALLCAPS 略語・allow-list に登録した語は除外。
- **coinage** — Sudachi 形態素解析で辞書外の複合語（不自然な造語・`slop軸` のような混種語も含む）を検出。
- **calque**（順次）— 英語動詞を「する」に接ぐ code-switching。

**木下原則の検査**（機械が言い切れる判定だけ — HARD）

- **kinoshita** — 一文の長さ・読点過多・ですます/である混在・慣用二重否定・ぼかし連発・指示語連鎖・「の」3 連鎖・冗長表現（`することができ`→`できる`）。文頭接続詞の連発（また/さらに/そして…）は advisory（LLM が生成した文の指紋・報告のみ）。

slop の検出器（codemix / coinage / calque）は候補を flag するだけで判定しない。domain か gratuitous かの裁定は呼び出し側（LLM-judge か人）が下す。kinoshita だけは機械判定が最終（blocking 可）。

## machine 界面 — `check --format json`（Tier 3・judge 連携）

全 finding を構造化 JSON で emit する。severity は `error`（HARD・exit 1 に数える）と `advisory`（locate 候補）の二値。「木で縛る」の正しい適用先は散文でなく **judge の入出力** — correo が座標を渡し、judge は structured output（JSON schema 制約）で分類を返す:

```sh
correo check --format json | your-judge --schema three-way.json
```

```json
{ "version": 1,
  "findings": [
    { "detector": "kinoshita", "rule": "sentence-length", "file": "a.md", "line": 3,
      "severity": "error", "message": "一文 128 字 (> 100) — 文を切る（一文一義）" },
    { "detector": "coinage", "rule": "dictionary-coinage", "file": "a.md", "line": 7,
      "severity": "advisory", "message": "「構造腕」は辞書見出し語でない複合 — …",
      "data": { "compound": "構造腕", "components": ["構造", "腕"] } }
  ],
  "summary": { "error": 1, "advisory": 1 } }
```

## roadmap — 木下原則の被覆計画

- **Tier 1（HARD）** — 実装済み＝上記 kinoshita。
- **Tier 2（MIX・proxy）** — 逆茂木の proxy（文頭の連体修飾チェーン長）→ flag のみ、judge が確認。
- **Tier 3（VIBE・座標のみ）** — トピックセンテンス・事実と意見・スリカエ → 段落第一文等の座標を構造化出力して LLM-judge に渡す（correo は判定しない）。
- 係り受けが要る原則（主述近接・修飾語順）は形態素の外 — proxy 化できた分だけ Tier 2 へ。

## install

formula は correo repo 内（`Formula/correo.rb`）が正本。repo の名前が `homebrew-*` でないため tap は URL 明示形で足す:

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
# これだけで動く: cwd 以下の *.md を全部検査（.gitignore 準拠・correo.toml を自動発見）
correo check

# 特定の file だけ / judge 連携（機械可読 JSON）
correo check draft.md
correo check --format json | your-judge

# 低レベルの単体検出器（diff-ratchet 等の組み込み用）
echo '本文に framework や pipeline を混ぜた段落。' | correo codemix --threshold 8
git diff -U0 | correo coinage --diff --strict --advisory
correo kinoshita report.md
```

`check` の指摘は `file:line: [検出器/規則] 説明` の一行形式。exit 1 になるのは kinoshita の
HARD 違反だけで、codemix と coinage(strict) は advisory（judge へ渡す候補）として数える。
`--no-default-features` でビルドすると coinage を外した純 codemix になる（Sudachi 依存なし）。

## 設定 — correo.toml（自動発見・無くても動く）

`correo check` は cwd から根へ辿って最初の `correo.toml` を読む。優先順位は CLI flag > correo.toml > 既定値。

```toml
allow = [       # judge（人か LLM）の裁定を経た語だけを登録する — 造語の逃げ場にしない
  "correo",     # 製品名
  "ルー語",      # 俗称として定着（言い換えると通じない）
]

[codemix]
threshold = 8.0

[kinoshita]
max-sentence = 100
max-ten = 4
```

一回きりの言及は登録せず、その場で抑制する（biome-ignore と同じ役割の分担）:

```markdown
<!-- correo-ignore -->
直後の行の指摘は全部抑制される。
規則名や検出器名で絞れる。 <!-- correo-ignore no-chain coinage -->
```

## 低レベル検出器の出力

`codemix` は密度が閾値を超えた段落を `⚑ L{行}: N latin/100字` で列挙（常に exit 0・advisory）。`coinage --strict` は辞書外複合を候補列挙し、`--advisory` 無しなら違反時 exit 1。

**codemix の判定条件**: JA 40字未満の段落は判定しない（密度が統計的に無意味なため）。ただし**連続する箇条書きは 1 まとまりに集計**して条件を越えさせる（LLM slop は短い bullet に湧くため。空行区切りの bullet の列も 1 単位）。code fence・inline code・URL・表・見出し行は地の文から除外して数える。短い段落に紛れる混種語・造語は coinage が段落長と無関係に捕まえる。

## license

MIT OR Apache-2.0。同梱する SudachiDict は Apache-2.0（`dict/LICENSE-SudachiDict`）。
