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

**台帳記法の漏出 — notation**（advisory・per-instance）

- 出自: 台帳体（研究 registry・記録の来歴ログ）の記号は、読者向け散文へ複写されると文の接続・論理・列挙の役目を持ったまま漏れる。2026-07-11 に qoed ポートフォリオ HTML 1 枚で実測した内訳は、矢印 18 件・全角イコール 10 件・括弧内 3 連結 5 件である。
- **arrow** — 矢印記号（`→`・`⇒`・`⟹`）が地の文に出現する場合。fence/inline code は言及なので対象外。
- **fullwidth-eq** — 連結用の全角イコール `＝`（片側以上が日本語文字のもの）。両側とも非日本語の数式的な使用は対象外。
- **paren-nakaten** — 丸括弧の中を中点で 3 要素以上つなぐ書き方。全角・半角どちらの括弧も対象。
- 見送り: 波ダッシュ/チルダの「約」用法（数値の直前に付く略記）は判定が濁るため見送った。
- 中点二重ダッシュの密度は rhetoric（下記）が既に持つため、ここでは扱わない。

**助数詞の欠落 — counter**（advisory・per-instance）

- 出自: notation と同事例（2026-07-11・qoed 句レベル指摘）。台帳体は「43 cell」「10 track」のように数字とラテン名詞を助数詞なしで直結する記法を常用する。台帳の内部では省略として正当だが、読者向け散文へ複写されると「43 cell を」のように助詞へ直接つながり、和文としての資格を失ったまま漏れる。notation とは検査する性質が別（notation は記号の文法的役割、counter は数字とラテン名詞の直結そのもの）なので同じ home に畳まない。
- **missing-counter** — 数字（半角）＋空白＋ラテン語幹の直後に、空白 0–1 個を挟んで日本語の助詞（を・が・は・で・に・と・へ・の）または句読点（。、！？）が続く場合。
- 除外: 単位・計量記号（ms・s・bits・byte・dB・JA 等）、日付・ID のハイフン連結数字（`2026-07-09` の `09`）、直後がアンダースコアで続く識別子の構成要素。数式文脈（`d=4`・`N=8`）は「数字＋空白＋ラテン語幹」の形自体に合致しないため構造上対象外。
- 較正（qoed ポートフォリオ HTML 抽出 corpus）: 陽性 3/3（「43 cell を」「8 cell の」「10 track を」）・ダッシュや括弧に続く非陽性 27 件は沈黙。correo README では該当 0 件。

**文の資格 — completeness**（advisory・prose_units 単位）

- 段落や箇条書きの項が、文の終端記号で閉じているかを検査する。見出し・表のセル・コード・URL 行は prose.rs の共通除外規約に従い対象外である。台帳的な体言止めの箇条書きには正当な用例があるため、severity は advisory 固定とし、最終判定は judge に委ねる。地の文の段落は折返しを 1 単位として末尾だけを見る一方、箇条書きは行ごとに独立した項目として判定する。

**読者軸 — register**（`--register internal|external`・correo.toml の `register` でも設定可・既定 `internal`）

- correo の全検出器は既定で **internal 読者**（書き手と統制語彙を共有するチーム）を仮定する。`external`（広い読者・顧客・審査者 等）を指定すると、下記 **jargon-export** が追加で発火する。他の検出器の挙動は register に依存しない。

**内輪語の輸出 — jargon-export**（advisory・per-instance・`--register external` のときだけ発火）

- 出自: 2026-07-11 qoed ポートフォリオの実務事例。免除辞書（correo.toml の `allow`・coinage の `corpus`）が書き手プロジェクトの統制語彙表（taxonomy）一本であるとき、「書き手の語彙 = 許される語彙」が構造に焼き込まれる。internal 読者ではこれで正しい。external 読者では**逆転する** — 統制語彙表に載る語こそが「定義なしで輸出された内輪語」の最優先候補になる。共有語彙とは読者と共有された語彙であって、書き手の語彙表そのものではない。実測の契機は campaign・criterion 族・menu が broad readership へ定義なしで輸出された事例である。
- 語彙表 file（`--jargon-vocabulary`・correo.toml の `[jargon] vocabulary`。`--allow` と同形式・taxonomy.md のような `| \`term\` | home | gloss | ... |` 表）の**第 1 列の見出し語**（セル先頭トークン。空白/括弧の手前まで）を検査対象にする。セル全体から latin token を拾うと、home の path 断片や gloss 文中の語まで語彙と誤認して過検出になる。POVM/Fisher/SDP/qubit のような広く定着した標準語も拾ってしまう（実測 2026-07-11: taxonomy.md 全体では 191 語・先頭トークン限定では 40 語）。
- 各語の**初出**について、以下のどちらかを満たせば合格（advisory を出さない）:
  - **(a) 近傍の定義徴候** — 直後の括弧に日本語の言い換え（`campaign（測定の実施計画）`）、その逆順（`（測定の実施計画）campaign`）。全角/半角括弧のどちらも可。または同一行内の「とは」定義文（`campaign とは測定の実施計画である`）。
  - **(b) 文書内の用語表** — markdown table か HTML table・dl があり、その第 1 列（`<td>`/`<th>`/`<dt>` の先頭セル）にその語が載っている。
- `internal`（既定）では jargon-export は発火しない。同じ語彙表は従来どおり codemix/coinage の免除リストとして働く（allow-list としての役割は不変。**同じ file が register で役割を反転する**、という設計）。
- 見送り: カタカナ化した内輪語の異表記対応（例「メニュー」を `menu` の変種として拾う）は見送った。音訳が非決定的で（`campaign`→`キャンペーン`、`menu`→`メニュー`、複合語の promotion 規則が語ごとに割れる）、既存コードベースに逆音訳機構が無いためである。将来案: Sudachi の読み仮名正規化（かな→ラテン語源）を使った近似照合、または語彙表側にカタカナ異表記を明示登録させる拡張。

**修辞密度 — rhetoric**（文体の指紋・全て advisory・文書単位）

- 装飾メタファーの密度（「発射台」「密輸」「解き放つ」級の直訳比喩 — 語彙は binary に持たず `lexicons/metaphor-lex.tsv` の **data**。採用 28 語幹と棄却 35 語の裁定記録が file 内に同居し、検出時は語彙と書き換え指示を列挙して judge へ渡す）。対立法「ではなく」と中点二重ダッシュの挿入と見出し副題の統一率は、**2 指標以上の同時超過でだけ** 1 件に合成する。中点二重ダッシュ自体は notation/arrow と別の性質を見る。こちらが見るのは文体の指紋（密度）で、記号が果たす役割の肩代わりではない。単独では正当な技法であり、rate 単独の発火は人間の名文を先に撃つと反証で実測された（青空文庫の古典は生成文 corpus より対立法率が上に座る）。30 文未満は計測不能として沈黙する。

**語彙の裁定 — deny**

- **slop-phrase（組み込み）** — LLM 常套句（`架け橋となる`・`可能性を解き放つ`・`いかがでしたか` 等）を出荷時の裁定として HARD で弾く。Vale の style 配布物に相当する「意見のある既定」— 解除はその語を allow に書く（拒否権は利用者にある）。
- 同じ組み込み辞書に、個別に「書き直す」と裁定済みの混種複合（英語幹＋漢字接辞: `closed表`・`open表`・`copies数`・`software層`・`判定家族`）も 2026-07-11 に合流した。由来は hype/marketing calque でなく house の確定裁定だが、機構は共通の ratchet（再侵入を機械が阻止する）である。
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
| conditional（用語の初出定義） | jargon-export（`--register external`。ただし Vale の conditional は「用語 A が出たら用語 B も出よ」の対応関係を見るのに対し、correo は「登録語彙が定義徴候なしで出たか」を見る単方向の検査 — 完全な対応ではない） |
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
| deny/slop-phrase | 語彙の裁定 | 語幹辞書（組み込み 19 語＝slop 定型句 14 語＋混種複合 5 語） | 独自＋preset-ai-writing hype 辞書から精度選別（混種複合 5 語は 2026-07-11・qoed 句レベル指摘の裁定済み再侵入防止: `closed表`・`open表`・`copies数`・`software層`・`判定家族`） | error |
| deny/denied-term | 語彙の裁定 | user 辞書（correo.toml） | judge の確定裁定 | error |
| notation/arrow | 台帳記法の漏出 | regex（→・⇒・⟹） | 独自（2026-07-11・qoed ポートフォリオ HTML 事例。台帳体の読者面への漏出。較正: 陽性 corpus 18/18） | advisory |
| notation/fullwidth-eq | 台帳記法の漏出 | regex（＝の片側以上が日本語文字） | 独自（同上。較正: 陽性 corpus 9/10・数式的使用「x＝1」は対象外） | advisory |
| notation/paren-nakaten | 台帳記法の漏出 | 括弧内「・」2+ 個（3要素以上） | 独自（同上。較正: 陽性 corpus 4/5・inline code 言及は除外） | advisory |
| completeness/unterminated-prose | 文の資格 | 散文単位の末尾が文末記号でない | 独自（2026-07-11・同事例。台帳的な体言止め箇条書きは正当がありうるため advisory 固定・judge へ回す） | advisory |
| counter/missing-counter | 助数詞の欠落 | regex（数字＋空白＋ラテン語幹が助詞・句読点に直結。単位語／日付連結／識別子継続は除外） | 独自（2026-07-11・qoed 句レベル指摘。台帳の圧縮表記「43 cell」が読者向け散文へ複写される事故。較正: qoed ポートフォリオ HTML 抽出 corpus で陽性 3/3・非陽性〔ダッシュ・括弧続き〕27 件は沈黙・correo README 陰性 0 件） | advisory |
| jargon/jargon-export | 内輪語の輸出 | 語彙表第 1 列の見出し語の初出に定義徴候（近傍括弧言い換え／とは文／用語表）が無いか | 独自（2026-07-11・register 軸導入。`--register external` でのみ発火。較正: qoed ポートフォリオ HTML で taxonomy.md 先頭トークン限定 40 語中 6→5 件が残存・内訳は README §読者軸参照） | advisory |

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

formula は correo repo 内（`Formula/correo.rb`）が正本。プリビルド binary を release から取るのでコンパイル不要（数秒）。repo 名が `homebrew-*` でないため tap は URL 明示形で足す:

```sh
brew tap fuyutarow/correo https://github.com/fuyutarow/correo.git
brew install fuyutarow/correo/correo   # 対応: macOS arm64/x86_64・linux x86_64
```

binary は ~1.8MB と小さい — Sudachi 辞書（~207MB）は同梱せず **CLI が管理**する。coinage（造語検出）を使うときだけ、初回に一度:

```sh
correo setup   # ~/.cache/correo へ辞書を取得。以後は環境変数なしで coinage が動く
```

辞書不要の検出器（codemix / calque / readability / rhetoric / structure / density）は setup なしで動く。ソースから入れる場合は `cargo install --git https://github.com/fuyutarow/correo`（`--features coinage` 既定）。

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
correo check build/report.html   # HTML も可（tag を剥いで全検出器・行番号は source 行）

# 低レベルの単体検出器（diff-ratchet 等の組み込み用）
echo '本文に framework や pipeline を混ぜた段落。' | correo codemix --threshold 8
git diff -U0 | correo coinage --diff --strict --advisory
correo readability report.md

# 読者軸: 顧客・審査者など外部読者向けの成果物は external を指定する（jargon-export が発火）
correo check deck.html --register external --jargon-vocabulary docs/handbook/taxonomy.md
```

`check` の指摘は `file:line: [検出器/規則] 説明` の一行形式。exit 1 に数えるのは error
（readability の HARD 違反と deny）。codemix・coinage・calque・structure・density・rhetoric・
jargon は advisory（judge へ渡す候補）として数える。
`--no-default-features` でビルドすると coinage を外した純 codemix になる（Sudachi 依存なし）。
HTML 内の `<!-- correo-ignore -->` は行内抑制のためのコメントだが、tag 剥ぎで一緒に消えるため効かない（build 生成物からの検出を想定した仕様で、抑制は source 側で行う前提）。

## 設定 — correo.toml（自動発見・無くても動く）

`correo check` は cwd から根へ辿って最初の `correo.toml` を読む。優先順位は CLI flag > correo.toml > 既定値。形式が TOML なのは Rust 圏の慣習（Cargo.toml/mise.toml と同じ）に合わせた判断で、裁定理由をコメントで残せることが allow 運用の前提になっている。未知のキーは黙殺せずパースエラーにする（打ち間違いの黙殺は事故のもと）。エディタ補完・検証は先頭の `#:schema` directive（taplo / tombi が読む — biome.json の `$schema` と同じ役割）で効き、schema は `schemas/correo.schema.json` にある。

```toml
#:schema https://raw.githubusercontent.com/fuyutarow/correo/main/schemas/correo.schema.json

allow = [       # judge（人か LLM）の裁定を経た語だけを登録する — 造語の逃げ場にしない
  "correo",     # 製品名
  "ルー語",      # 俗称として定着（言い換えると通じない）
]

# 読者軸（既定 internal）。external では allow と同じ語彙表が jargon-export の検査対象へ役割反転する。
register = "internal"

[codemix]
threshold = 8.0

[readability]
max-sentence = 100
max-ten = 4

# 実在語彙表（coinage の候補から実在語を消して judge へのノイズを減らす）
[coinage]
corpus = "~/.cache/correo/corpus.tsv"   # mise run setup:corpus が配置

# jargon-export（register="external" のときだけ発火）の検査対象語彙表。--allow と同形式で
# 第 1 列の見出し語だけを候補にする（qoed では docs/handbook/taxonomy.md）。
[jargon]
vocabulary = "docs/handbook/taxonomy.md"

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
