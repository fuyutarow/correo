class Correo < Formula
  desc "Lint for Japanese LLM slop: code-mixing density and dictionary coinage"
  homepage "https://github.com/fuyutarow/correo"
  # NOTE: まだ release していないため url/sha256 は placeholder。release を切る手順:
  #   git tag v0.2607.0 && gh release create v0.2607.0 --generate-notes
  #   curl -sL https://github.com/fuyutarow/correo/archive/refs/tags/v0.2607.0.tar.gz | shasum -a 256
  #   → 下の 0000… を置換。プリビルド配布（bottle）は .github/workflows/release.yml が担う。
  url "https://github.com/fuyutarow/correo/archive/refs/tags/v0.2607.0.tar.gz"
  sha256 "0000000000000000000000000000000000000000000000000000000000000000"
  license any_of: ["MIT", "Apache-2.0"]
  head "https://github.com/fuyutarow/correo.git", branch: "main"

  depends_on "rust" => :build

  def install
    # coinage feature を明示（既定 default だが依存を確実に引き込む）。
    system "cargo", "install", "--features", "coinage", *std_cargo_args
    # メタファー語彙表（data・小さい）は同梱。binary は exe 相対 (bin/../share/correo/) で解決する。
    # Sudachi 辞書（~207MB）は同梱しない — CLI が管理する（`correo setup` が ~/.cache へ取得）。
    (share/"correo").install "lexicons/metaphor-lex.tsv"
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
    # メタファー語彙表が exe 相対で同梱・解決されるか。
    assert_path_exists share/"correo/metaphor-lex.tsv"
    # codemix は辞書不要（純 locate）— binary が動く実証。
    assert_match "CODEMIX", pipe_output("#{bin}/correo codemix", "framework を混ぜた日本語の段落。\n")
  end
end
