// Sudachi 辞書(core) を sudachidict-core wheel から ~/.cache/correo へ抽出
// （coinage の live エンジン用・resolve_dict_dir の最終 fallback と一致・要 uv）。
//
// Migrated verbatim from the `setup:sudachidict` mise task body (wiring-mise-tasks
// contract: a TOML-embedded body with control flow cannot be imported or tested).
import { $ } from "bun";

const pyScript = `
import sudachidict_core, sudachipy, pathlib, shutil
dst = pathlib.Path.home() / '.cache' / 'correo'
shutil.copy(pathlib.Path(sudachidict_core.__file__).parent / 'resources' / 'system.dic', dst / 'system.dic')
for f in (pathlib.Path(sudachipy.__file__).parent / 'resources').iterdir():
    if f.is_file():
        shutil.copy(f, dst / f.name)
print('correo dict ready:', dst)`;

await $`mkdir -p ~/.cache/correo`;
await $`uv run --with sudachipy --with sudachidict-core python -c ${pyScript}`;
