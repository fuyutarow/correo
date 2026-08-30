// version の z を進める（同月なら z+1・月替わりなら 0.YYMM.0）。
// tag より既に進んでいれば no-op。CalVer next の計算はこのタスクだけが持つ
//
// Migrated verbatim from the `bump` mise task body (wiring-mise-tasks contract:
// a TOML-embedded body over 10 lines cannot be imported or tested).
import { $ } from "bun";

const tag = (await $`git describe --tags --abbrev=0 --match 'v*'`.text()).trim();
const tagVer = tag.replace(/^v/, "");
const ver = (await $`grep -m1 '^version' Cargo.toml | cut -d'"' -f2`.text()).trim();

// Bun's printf builtin does not itself decode \n escapes (measured 2026-08-30) — this format
// string embeds LITERAL newline bytes (via the template literal's own \n escape) rather than
// the two-character "\n" GNU printf would decode, so sort -V still sees two lines.
const latest = (
  await $`printf '%s
%s
' ${tagVer} ${ver} | sort -V | tail -n1`.text()
).trim();

if (ver !== tagVer && latest === ver) {
  console.log(`already bumped: ${ver} > ${tagVer}`);
  process.exit(0);
}

const yymm = (await $`date +%y%m`.text()).trim();
let next: string;
if (tagVer.startsWith(`0.${yymm}.`)) {
  const z = Number(tagVer.slice(tagVer.lastIndexOf(".") + 1));
  next = `0.${yymm}.${z + 1}`;
} else {
  next = `0.${yymm}.0`;
}

// Bun.which, NOT `$`command -v``: Bun's shell has no `command` builtin, so that form returns
// exit 1 even for a binary that exists (measured 2026-08-30). It would reinstall cargo-edit on
// every run. Bun.which walks PATH the way the original `command -v` did.
if (Bun.which("cargo-set-version") === null) {
  await $`cargo install cargo-edit`;
}
await $`cargo set-version ${next}`;
await $`cargo check --quiet`;
console.log(`bumped: ${ver} -> ${next}`);
