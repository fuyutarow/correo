# correo 統制語彙 registry（allow-list）

この repo 自身を `correo check --allow docs/vocab.md` で lint するための登録簿（dogfood）。
形式は correo の allow 規約どおり: `| \`term\` | gloss |` の行だけが登録される。
codemix は行内の latin token を、coinage は複合語の表層をここから exempt する。
**登録は judge（人/LLM）の裁定を経た語のみ** — 未裁定の新語は advisory に現れて然るべき。

## 製品・検出器名（pinned）

| term | gloss |
|---|---|
| `correo` | 本ツール名 |
| `codemix` | ルー語密度検出器 |
| `coinage` | 造語検出器 |
| `kinoshita` | 木下 HARD 層検出器 |
| `calque` | 動詞カルク検出器（順次） |
| `sudachi` | 形態素解析エンジン |
| `prh` | 確定造語の焼き付け先 |

## domain 用語（架構の語彙）

| term | gloss |
|---|---|
| `slop` | LLM が混ぜる言語の不自然さ |
| `locate` | 候補を挙げる層（判定しない） |
| `judge` | 分類を下す層（LLM/人） |
| `llm-judge` | judge の LLM 実装 |
| `advisory` | 報告のみ・exit に数えない |
| `blocking` | exit 1 に数える |
| `severity` | error / advisory の二値 |
| `finding` | 検出 1 件（JSON 界面の単位） |
| `schema` | judge の structured output 制約 |
| `structured output` | schema 制約付き生成 |
| `allow-list` | この登録簿 |
| `latin` | ラテン文字 token |
| `code-switching` | 言語切替（カルクの上位概念） |
| `domain` | 3-way 分類: 分野語 |
| `pinned` | 3-way 分類: 固定語 |
| `gratuitous` | 3-way 分類: 不要な混入 |
| `tier` | 木下被覆の層（HARD/MIX/VIBE） |
| `proxy` | 係り受け不要の近似指標 |
| `flag` | 候補として挙げる |
| `emit` | 構造化出力する |
| `fence` | code block（判定除外） |
| `inline` | inline code（判定除外） |
| `bullet` | 箇条書き行（行群に集計） |
| `exit` | 終了コード |
| `output` | 出力 |
| `code` | コード |

## 家造語（意図した coined terms — coinage の exempt）

| term | gloss |
|---|---|
| `機械床` | 機械判定できる lint 層（本 README の中心語） |
| `判定床` | 判定対象に入る最小条件 |
| `ルー語` | 地の文への latin 混ぜ（ルー大柴に由来する俗称） |
| `ルー語密度` | latin/100字 の密度 |
| `木下軸` | 木下是雄由来の検査軸 |
| `行群` | 連続 bullet を 1 単位に集計したもの |
| `層別` | tier ごとの分類 |

## 自然な複合だが Sudachi C 単位に無い語（strict tier の既知 FP — 裁定: 自然）

| term | gloss |
|---|---|
| `日本語実用文` | 日本語の実用文 |
| `登録語` | 登録された語 |
| `生成文` | 生成された文 |
| `字未満` | 〜字未満（数量表現の断片） |
| `辞書見出し語` | 辞書の見出し語 |
| `辞書外複合` | 辞書に無い複合語 |
