# correo 統制語彙の登録簿（allow-list）

この repo 自身を lint するための登録簿（`mise run lint:self` が README と本ファイルを検査する）。
形式は correo の allow 規約どおり: `| \`term\` | gloss |` の行だけが登録される。
codemix は行内の latin token を、coinage は複合語の表層をここから除外する。
**登録できるのは judge（人か LLM）の裁定を経た語だけ** — 未裁定の新語は advisory に現れるのが正しい。

## 登録の規律 — 登録は造語の逃げ場ではない

**登録とは「この語を外の読者に対して弁護できる」という裁定であって、書き直しから逃げる手段ではない。**
書き直せる語は書き直す。撤回の記録（2026-07-09）: `機械床`・`判定床`・`木下軸`・`行群`・`層別` を
独自の造語のまま一括登録したが、judge（人間）に却下された — README を標準語へ書き直し、登録を
撤回した。correo が正しく flag した語を登録で黙らせるのは、locate と judge の分業の敗北である。

## 製品・検出器の名前

| term | gloss |
|---|---|
| `correo` | 本ツール名 |
| `codemix` | ルー語の密度の検出器 |
| `coinage` | 造語の検出器 |
| `kinoshita` | 木下原則の検出器 |
| `calque` | 動詞カルクの検出器（順次） |
| `sudachi` | 形態素解析エンジン |
| `prh` | 確定した造語の禁止先 |

## 分野の用語

| term | gloss |
|---|---|
| `slop` | LLM が混ぜる言語の不自然さ |
| `locate` | 候補を挙げる係（判定しない） |
| `judge` | 分類を下す係（LLM か人） |
| `llm-judge` | judge の LLM 実装 |
| `advisory` | 報告のみ・終了コードに数えない |
| `blocking` | 終了コードに数える |
| `severity` | error / advisory の二値 |
| `finding` | 検出 1 件（JSON 出力の単位） |
| `schema` | judge の出力に課す構造の制約 |
| `structured output` | 構造を制約した生成 |
| `allow-list` | この登録簿 |
| `latin` | ラテン文字の token |
| `code-switching` | 言語の切り替え（カルクの上位概念） |
| `domain` | 3-way 分類: 分野の用語 |
| `pinned` | 3-way 分類: 固定した語 |
| `gratuitous` | 3-way 分類: 不要な混入 |
| `tier` | 木下原則の被覆の段階（HARD/MIX/VIBE） |
| `proxy` | 係り受けを使わない近似の指標 |
| `flag` | 候補として挙げる |
| `emit` | 構造化して出力する |
| `fence` | code block（判定から除外） |
| `inline` | inline code（判定から除外） |
| `bullet` | 箇条書きの行 |
| `exit` | 終了コード |
| `output` | 出力 |
| `code` | コード |

## 書き換えずに使い続けると裁定した語

| term | gloss |
|---|---|
| `ルー語` | 地の文に latin を混ぜる話し方。ルー大柴に由来し俗称として定着（言い換えると通じない） |
| `ルー語密度` | latin/100字 の密度 |

## 実在する複合語だが Sudachi の C 単位に無い語（strict の既知の誤検出 — 裁定: 自然な日本語）

| term | gloss |
|---|---|
| `日本語実用文` | 日本語の実用文 |
| `実用文` | 実用の文章（実在する語） |
| `見出し語` | 辞書の項目の語（実在する語） |
| `混種語` | 言語学の用語（latin と和語の混成） |
| `登録語` | 登録された語 |
| `生成文` | 生成された文 |
| `字未満` | 〜字未満（数量の言い方の断片） |
| `辞書見出し語` | 辞書の見出し語 |
| `辞書外複合` | 辞書に無い複合語 |
