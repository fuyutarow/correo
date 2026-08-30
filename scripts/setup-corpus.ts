// coinage の corpus 照合用語彙表を構築（SudachiDict full lex CSV の全表層 ∪ jawiki 記事タイトル
// → ~/.cache/correo/corpus.tsv）。lex CSV は辞書編纂者が実在を裁定済みの語彙 — core 辞書の C 単位
// に無い複合（使用例・実用文）を実在側へ救い、真の造語（機械床）だけを error に残す。
// BCCWJ 長単位語彙表（doi.org/10.15084/00003212）へ差し替え可
//
// Migrated verbatim from the `setup:corpus` mise task body (wiring-mise-tasks contract:
// a TOML-embedded body over 10 lines cannot be imported or tested).
import { $ } from "bun";

await $`mkdir -p ~/.cache/correo`;
const T = (await $`mktemp -d`.text()).trim();

await $`curl -sL https://dumps.wikimedia.org/jawiki/latest/jawiki-latest-all-titles-in-ns0.gz | gunzip | sed 's/_/ /g' > ${T}/words`;

for (const f of ["core_lex", "notcore_lex"]) {
  await $`curl -sL "http://sudachi.s3-ap-northeast-1.amazonaws.com/sudachidict-raw/20260428/${f}.zip" -o ${T}/${f}.zip`;
  await $`unzip -p ${T}/${f}.zip | cut -d, -f1 >> ${T}/words`;
}

await $`sort -u ${T}/words > ~/.cache/correo/corpus.tsv`;
await $`rm -rf ${T}`;
await $`wc -l ~/.cache/correo/corpus.tsv`;
