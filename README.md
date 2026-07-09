# correo

日本語実用文の lint — LLM slop（言語の不自然さ）と、木下是雄『理科系の作文技術』の原則のうち**機械的に判定できるもの**を検査する。**事実性・hype は対象外**。

検出器は二系統:

**LLM slop の検出**（機械が混ぜた非母語的な日本語。候補を挙げるだけで判定しない）

- **codemix** — 地の文の latin/100字 密度（ルー語）。識別子・ALLCAPS 略語・allow-list に登録した語は除外。
- **coinage** — Sudachi 形態素解析で辞書外の複合語（不自然な造語・`slop軸` のような混種語も含む）を候補として挙げる。corpus（実在の語彙表）を設定すると、**実在が証明できた複合（`物理層`）を候補から消して** judge へ渡すノイズを減らす。不在は error にしない — `使用例` のような生産的複合はどんな有限の語彙表にも載らないため、**不在≠造語**。造語の確定は judge が下し、その裁定は `[deny]` が永続化する（`機械床` の再侵入は HARD で落ちる）。
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

## 位置づけ — Vale の DevX を日本語で

英語圏の文章 lint は [Vale](https://vale.sh)（Go 製・単一バイナリ・マークアップ対応・オフライン）が事実上の標準だが、**Vale は形態素解析を持たず日本語には機能しない**。日本語の定番は textlint（Node.js 製・JSON 設定・プラグイン構成）だった。correo はこの隙間に立つ。Vale と同じ配布の哲学 — 単一バイナリ・オフライン・マークアップ対応・設定 1 ファイル・語彙規則をコードなしで書ける — を日本語で提供する。形態素解析は Sudachi が、文脈の判定は LLM-judge への構造化出力が受け持つ。

## roadmap — 木下原則の被覆計画

- **Tier 1（HARD）** — 実装済み＝上記 kinoshita。
- **Tier 2（MIX・proxy）** — 逆茂木の proxy（文頭の連体修飾チェーン長）→ flag のみ、judge が確認。
- **Tier 3（VIBE・座標のみ）** — トピックセンテンス・事実と意見・スリカエ → 段落第一文等の座標を構造化出力して LLM-judge に渡す（correo は判定しない）。
- 係り受けが要る原則（主述近接・修飾語順）は形態素の外 — proxy 化できた分だけ Tier 2 へ。

## roadmap — coinage の証拠強化

- **語彙表の Bloom filter 化** — 現在の語彙表は ~390万行のテキスト（数十 MB・任意設定）。Bloom filter に落とせば**数 MB を binary か formula に同梱**でき、利用者は何も取得せず実在照合が効く（偽陽性は「候補を稀に消す」安全側にしか倒れない）。
- **実文 n-gram による error 昇格** — 語彙表（lexicon）の不在は造語の証拠にならない。しかし**実文コーパスの頻度ゼロ**は強い証拠になる（`使用例` は実文に大量出現し `機械床` はゼロ）。BCCWJ n-gram か jawiki 全文から複合語 n-gram 集合を構築できれば、judge を待たない error 昇格が正当化される。それまで造語の確定は judge → `[deny]`。

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
# 設定の雛形を作る（無くても動く）
correo init

# これだけで動く: cwd 以下の *.md を全部検査（.gitignore 準拠・correo.toml を自動発見）
correo check

# 機械的に安全な違反を直して書き戻す（現在: することができ→でき）
correo check --write

# 特定の file だけ / judge 連携（JSON）/ GitHub Actions の PR inline 注釈
correo check draft.md
correo check --format json | your-judge
correo check --format github   # ::error / ::notice を emit — PR の該当行に注釈が付く

# 低レベルの単体検出器（diff-ratchet 等の組み込み用）
echo '本文に framework や pipeline を混ぜた段落。' | correo codemix --threshold 8
git diff -U0 | correo coinage --diff --strict --advisory
correo kinoshita report.md
```

`check` の指摘は `file:line: [検出器/規則] 説明` の一行形式。exit 1 になるのは kinoshita の
HARD 違反だけで、codemix と coinage(strict) は advisory（judge へ渡す候補）として数える。
`--no-default-features` でビルドすると coinage を外した純 codemix になる（Sudachi 依存なし）。

## 設定 — correo.toml（自動発見・無くても動く）

`correo check` は cwd から根へ辿って最初の `correo.toml` を読む。優先順位は CLI flag > correo.toml > 既定値。形式が TOML なのは Rust 圏の慣習（Cargo.toml/mise.toml と同じ）に合わせた判断で、裁定理由をコメントで残せることが allow 運用の前提になっている。未知のキーは黙殺せずパースエラーにする（打ち間違いの黙殺は事故のもと）。エディタ補完・検証は先頭の `#:schema` directive（taplo / tombi が読む — biome.json の `$schema` と同じ役割）で効き、schema は `schemas/correo.schema.json` にある。

```toml
#:schema https://raw.githubusercontent.com/fuyutarow/correo/main/schemas/correo.schema.json

allow = [       # judge（人か LLM）の裁定を経た語だけを登録する — 造語の逃げ場にしない
  "correo",     # 製品名
  "ルー語",      # 俗称として定着（言い換えると通じない）
]

[codemix]
threshold = 8.0

[kinoshita]
max-sentence = 100
max-ten = 4

# 実在語彙表（coinage の候補から実在語を消して judge へのノイズを減らす）
[coinage]
corpus = "~/.cache/correo/corpus.tsv"   # mise run setup:corpus が配置

# deny ＝ judge の確定裁定の永続 cache（値は書き直しの案・HARD＝exit 1）。
# 造語の最終判定は意味の領分で機械には下せない — judge が却下した語をここへ書くと
# 再侵入を機械が阻止する。実在するが使わない語（くだけた話し言葉等）にも使える。
[deny]
"機械床" = "「機械的に判定できる lint」など標準的な言い方へ書き直す"
"ぶっちゃけ" = "くだけた話し言葉 — 「率直に言えば」等へ"
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
