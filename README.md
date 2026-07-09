# correo

日本語実用文の lint — LLM slop（言語の不自然さ）と、木下是雄『理科系の作文技術』の原則のうち**機械的に判定できるもの**を検査する。**事実性・hype は対象外**。

検出器は「検査する性質」で分ける。規則の出自 — 木下の章・textlint 移植・独自 — は module の名前で示さず、後述の表が規則ごとに記録する。

**言語の自然さ**（機械が混ぜた非母語的な日本語。候補を挙げるだけで判定しない）

- **codemix** — 地の文の latin/100字 密度（ルー語）。識別子・ALLCAPS 略語・allow-list に登録した語は除外。
- **coinage** — Sudachi 形態素解析で辞書外の複合語（不自然な造語・`slop軸` のような混種語も含む）を候補として挙げる。corpus（実在の語彙表）を設定すると、**実在が証明できた複合（`物理層`）を候補から消して** judge へ渡すノイズを減らす。不在は error にしない — `使用例` のような生産的複合はどんな有限の語彙表にも載らないため、**不在≠造語**。造語の確定は judge が下し、その裁定は `[deny]` が永続化する（`機械床` の再侵入は HARD で落ちる）。
- **calque** — 英語動詞を「する/される」に直接接ぐ code-switching（`deployする`・`inspireされた`）。狭義のみを決定論で扱う — 広義の翻訳調（が行われ 等）は丁寧な人間文書にも現れるため（qoed corpus で が行われ×12 を実測・2026-07-09）、judge の領分。

**可読性・トーンの床 — readability**（木下是雄に根拠を持つ規則だけを置く・HARD 中心）

- 一文の長さ・読点過多・ですます/である混在・慣用二重否定・ぼかし連発・指示語連鎖・「の」3 連鎖・冗長表現（`することができ`→`できる`）。advisory は漢字 7 連続と感嘆符。

**定型構造 — structure**（LLM layout の指紋・全て advisory）

- 「太字見出し＋コロン」が並ぶ箇条書き・同じ書き出しが続く文・文頭接続詞の連発・述語＋コロンで箇条書きへ接続（英語直訳調）。木下に無い規則なので readability とは別 home（2026-07-09 再分割）。

**情報密度 — density**（advisory）

- 圧縮率（bits/字）で薄い反復を検出。段落の近重複も字 bigram 類似で検出。

**修辞密度 — rhetoric**（文体の指紋・全て advisory・文書単位）

- 装飾メタファーの密度（「発射台」「密輸」「解き放つ」級の直訳比喩 — 語彙は binary に持たず `lexicons/metaphor-lex.tsv` の **data**。採用 28 語幹と棄却 35 語の裁定記録が file 内に同居し、検出時は語彙と書き換え指示を列挙して judge へ渡す）。対立法「ではなく」・「——」挿入・見出し副題の統一率は **2 指標以上の同時超過でだけ** 1 件に合成する。単独では正当な技法であり、rate 単独の発火は人間の名文を先に撃つと反証で実測された（青空文庫の古典は生成文 corpus より対立法率が上に座る）。30 文未満は計測不能として沈黙。

**語彙の裁定 — deny**

- **slop-phrase（組み込み）** — LLM 常套句（`架け橋となる`・`可能性を解き放つ`・`いかがでしたか` 等）を出荷時の裁定として HARD で弾く。Vale の style 配布物に相当する「意見のある既定」— 解除はその語を allow に書く（拒否権は利用者にある）。
- **denied-term** — correo.toml の `[deny]`（judge が確定した裁定の永続 cache）。

locate/judge の分業: readability の HARD と deny だけ機械判定が最終（exit 1）。他は候補を flag するだけで、domain か gratuitous かの裁定は呼び出し側（LLM-judge か人）が下す。

## machine 界面 — `check --format json`（Tier 3・judge 連携）

全 finding を構造化 JSON で emit する。severity は `error`（HARD・exit 1 に数える）と `advisory`（locate 候補）の二値。「木で縛る」の正しい適用先は散文でなく **judge の入出力** — correo が座標を渡し、judge は structured output（JSON schema 制約）で分類を返す:

```sh
correo check --format json | your-judge --schema three-way.json
```

```json
{ "version": 1,
  "findings": [
    { "detector": "readability", "rule": "sentence-length", "file": "a.md", "line": 3,
      "severity": "error", "message": "一文 128 字 (> 100) — 文を切る（一文一義）" },
    { "detector": "coinage", "rule": "dictionary-coinage", "file": "a.md", "line": 7,
      "severity": "advisory", "message": "「構造腕」は辞書見出し語でない複合 — …",
      "data": { "compound": "構造腕", "components": ["構造", "腕"] } }
  ],
  "summary": { "error": 1, "advisory": 1 } }
```

## 位置づけ — Vale の DevX を日本語で

英語圏の文章 lint は [Vale](https://vale.sh)（Go 製・単一バイナリ・マークアップ対応・オフライン）が事実上の標準だが、**Vale は形態素解析を持たず日本語には機能しない**。日本語の定番は textlint（Node.js 製・JSON 設定・プラグイン構成）だった。correo はこの隙間に立つ。Vale と同じ配布の哲学 — 単一バイナリ・オフライン・マークアップ対応・設定 1 ファイル・語彙規則をコードなしで書ける — を日本語で提供する。形態素解析は Sudachi が、文脈の判定は LLM-judge への構造化出力が受け持つ。

AI 特化の近縁ツールとも機構を突合した（2026-07・全ルールを採取して実測）。`@textlint-ja/preset-ai-writing` は表層 regex＋固定辞書の 5 規則で、kuromoji はコロン規則の語末品詞判定だけに使う。meiseki は textlint を包む Claude Code plugin（自前の解析コードなし）、slopless は英語専用。**形態素×辞書 membership の造語・混種語検出、圧縮率の情報密度、段落の近重複 — この 3 機構はどれも持たない**（correo 固有）。常套句辞書は preset-ai-writing と部分交差の別集合だったため、精度基準で選別した和集合を採る（人間の常用語 — 完全に・大幅に 等 — は実測 FP 報告があるため不採録）。

Vale の規則類型との対応（何が在り、何が roadmap か）:

| Vale extension point | correo |
|---|---|
| substitution / existence | `[deny]`（値=書き直し案）＋ 組み込み slop-phrase |
| spelling（語彙照合） | coinage（形態素＋辞書＋corpus 照合 — わかち書きの無い日本語版） |
| repetition | opener-repetition・connector-pileup |
| occurrence（頻度制限） | ぼかし連発・指示語連鎖・「の」3 連鎖 |
| readability | 文長・読点の数（日本語の実効指標） |
| conditional（用語の初出定義） | roadmap |
| capitalization（見出し体裁） | roadmap（体言止め統一 等） |
| Packages（style 配布） | 組み込み既定＋`[extends]` は roadmap |

## 規則台帳 — 性質×機構×出自

module 分割は性質で切り、出自はこの表が規則ごとに記す。出自を module の名前で示すのはやめた — bullet-template は木下の規則ではない（2026-07-09 再分割の理由）。

| 規則 | 性質 | 機構 | 出自 | severity |
|---|---|---|---|---|
| codemix/latin-density | 言語の自然さ | latin/100字 密度 | 独自（qoed 由来） | advisory |
| calque/verb-calque | 言語の自然さ | regex（latin＋する/され 接合） | 独自（qoed 設計） | advisory |
| coinage/dictionary-coinage | 言語の自然さ | Sudachi C 単位×辞書 membership×corpus 照合 | 独自 | advisory |
| readability/sentence-length | 可読性 | 文字数 | 木下（一文一義） | error |
| readability/max-ten | 可読性 | 読点数 | 木下 | error |
| readability/no-chain | 可読性 | regex＋題名 mask | 木下（「の」は 2 つまで） | error |
| readability/double-negative | 可読性 | regex | 木下（言い切り） | error |
| readability/hedge-pileup | トーン | regex・文内 2+ | 木下（言い切り） | error |
| readability/demonstrative-chain | 可読性 | regex・文内 3+ | 木下 | error |
| readability/verbose-potential | 可読性 | regex（`--write` 対応） | 木下（簡潔）。ja-no-redundant-expression 相当 | error |
| readability/style-mixing | トーン | 文末分類×文書集計 | 木下。no-mix-dearu-desumasu 相当 | error |
| readability/kanji-run | 可読性 | regex 7 連続+ | textlint 移植（max-kanji-continuous-len） | advisory |
| readability/exclamation | トーン | regex | 木下＋textlint（no-exclamation-question-mark） | advisory |
| structure/bullet-template | 定型構造 | 行 regex×3 連続 | 独自。no-ai-list-formatting 相当 | advisory |
| structure/opener-repetition | 定型構造 | 接頭 6 字×3 文連続 | 独自 | advisory |
| structure/connector-pileup | 定型構造 | regex×3 文連続 | 独自。ai-tech-writing-guideline 相当 | advisory |
| structure/colon-continuation | 定型構造 | ひらがな＋コロン→block 隣接 | preset-ai-writing 移植（Sudachi 不要化） | advisory |
| density/low-information-density | 情報密度 | deflate 圧縮率 < 11 bits/字 | 独自（Shannon） | advisory |
| density/near-duplicate | 情報密度 | 字 bigram 類似 ≥ 0.65 | 独自 | advisory |
| rhetoric/metaphor-density | 修辞密度 | 外部語彙表 data×rate（言及・引用・表は除外） | 独自（較正 fleet 2026-07-09・採用/棄却の裁定は語彙表内に記録） | advisory |
| rhetoric/stylometric-uniformity | 修辞密度 | 対立法/ダッシュ/副題統一の composite（2+ 指標） | 独自（同上・単独 rate 発火は反証により禁止） | advisory |
| deny/slop-phrase | 語彙の裁定 | 語幹辞書（組み込み 14 語） | 独自＋preset-ai-writing hype 辞書から精度選別 | error |
| deny/denied-term | 語彙の裁定 | user 辞書（correo.toml） | judge の確定裁定 | error |

## roadmap — 木下原則の被覆計画

- **Tier 1（HARD）** — 実装済み＝上記 readability。
- **Tier 2（MIX・proxy）** — 逆茂木の proxy（文頭の連体修飾チェーン長）→ flag のみ、judge が確認。
- **Tier 3（VIBE・座標のみ）** — トピックセンテンス・事実と意見・スリカエ → 段落第一文等の座標を構造化出力して LLM-judge に渡す（correo は判定しない）。
- 係り受けが要る原則（主述近接・修飾語順）は形態素の外 — proxy 化できた分だけ Tier 2 へ。
- 従来 textlint からの移植候補（Sudachi でより正確に作れる順）: 二重助詞・同一 token の連続（これはは）・ら抜き。
- 検出と報告の分離（slopless の density policy）: 単発は許し、窓あたり密度で severity を裁く報告エンジン。ぼかし・接続詞系の誤爆をさらに減らす。

## roadmap — coinage の証拠強化

- **語彙表の Bloom filter 化** — 現在の語彙表は ~390万行のテキスト（数十 MB・任意設定）。Bloom filter に落とせば**数 MB を binary か formula に同梱**でき、利用者は何も取得せず実在照合が効く（偽陽性は「候補を稀に消す」安全側にしか倒れない）。
- **実文 n-gram による error 昇格** — 語彙表（lexicon）の不在は造語の証拠にならない。しかし**実文コーパスの頻度ゼロ**は強い証拠になる（`使用例` は実文に大量出現し `機械床` はゼロ）。BCCWJ n-gram か jawiki 全文から複合語 n-gram 集合を構築できれば、judge を待たない error 昇格が正当化される。それまで造語の確定は judge → `[deny]`。

## 採らない道 — perplexity 系の検出（裁定の記録）

低 perplexity で slop を判定する路線（GLTR / DetectGPT・Binoculars = arXiv:2401.12070, ICML 2024）は採らない。

- それは**著者の推定**であって品質の判定ではない。本ツールは LLM と書いた文書を良くする道具で、「AI が書いた」こと自体は違反でない。
- 簡潔で予測しやすい文は良い技術文の特徴そのもの。神経モデルの perplexity では**木下に忠実な文書ほど低く**なり、真面目な仕様書を誤爆する。
- モデル同梱の重さと、生成側の偽装（temperature・burstiness 指示）との いたちごっこ。

「予測しやすさ」の品質側の核は `density` が持つ — 圧縮率は文字レベルの perplexity に等しく（Shannon: 圧縮長≒エントロピー）、モデル無しで反復を咎める。ただし実測では**良い文書の段落も術語の正当な反復で 12–14 bits/字 に沈む**（この README 自身で 11 件誤爆した記録）。安全に分離できるのは極端な反復（<11）だけ — 閾値 11 の保守運用とし、境界域は judge に委ねる。段落規模の圧縮率でも「良い技術文＝低エントロピー」問題からは逃げ切れない、が実測の結論。

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
correo readability report.md
```

`check` の指摘は `file:line: [検出器/規則] 説明` の一行形式。exit 1 に数えるのは error
（readability の HARD 違反と deny）。codemix・coinage・calque・structure・density・rhetoric
は advisory（judge へ渡す候補）として数える。
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

[readability]
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
