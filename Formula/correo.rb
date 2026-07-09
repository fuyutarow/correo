class Correo < Formula
  desc "Lint for Japanese LLM slop: code-mixing density and dictionary coinage"
  homepage "https://github.com/fuyutarow/correo"
  version "0.2607.1"
  license any_of: ["MIT", "Apache-2.0"]

  # プリビルド binary を release から取得する（コンパイル不要 = 数秒で入る）。binary は極小
  # (~1.8MB) — Sudachi 辞書 (~207MB) は同梱せず CLI が管理する（`correo setup`）。
  # release.yml が tag 契機で 3 target を build・upload。sha256 は v0.2607.1 の資産を実測 (2026-07-09)。
  on_macos do
    on_arm do
      url "https://github.com/fuyutarow/correo/releases/download/v0.2607.1/correo-v0.2607.1-aarch64-apple-darwin.tar.gz"
      sha256 "54295d04fd03a7bfd03aa6b09f2210fa9943850d596cb45077fce0ee028ebd99"
    end
    on_intel do
      url "https://github.com/fuyutarow/correo/releases/download/v0.2607.1/correo-v0.2607.1-x86_64-apple-darwin.tar.gz"
      sha256 "8837cb4019dac7a8a68e08bfe94eff3d1515da353e0cc0f192ea129692d3611c"
    end
  end
  on_linux do
    on_intel do
      url "https://github.com/fuyutarow/correo/releases/download/v0.2607.1/correo-v0.2607.1-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "9eaf05a0f1db00f4f7c34991726142beae4a3b07d3dca31d1a93fa52e2f254d2"
    end
  end

  def install
    bin.install "correo"
    # メタファー語彙表（data）— binary は exe 相対 (bin/../share/correo/) で解決する。
    (share/"correo").install "metaphor-lex.tsv"
  end

  def caveats
    <<~EOS
      coinage（造語検出）は Sudachi 辞書を要する。初回だけ取得する:
        correo setup
      （~/.cache/correo へ ~207MB を配置。以後は環境変数なしで coinage が動く）
      辞書不要の検出器（codemix / calque / readability / rhetoric / structure / density）は
      setup なしで動く。
    EOS
  end

  test do
    assert_match "correo", shell_output("#{bin}/correo --version")
    assert_path_exists share/"correo/metaphor-lex.tsv"
    # codemix は辞書不要（純 locate）— binary が動く実証。
    assert_match "CODEMIX", pipe_output("#{bin}/correo codemix", "framework を混ぜた日本語の段落。\n")
  end
end
