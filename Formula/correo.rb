class Correo < Formula
  desc "Lint for Japanese LLM slop: code-mixing density and dictionary coinage"
  homepage "https://github.com/fuyutarow/correo"
  # NOTE: correo をまだ release していないため、この 2 行だけ placeholder。
  #   手順: git tag v0.1.0 && gh release create v0.1.0 --generate-notes
  #         → `brew fetch --build-from-source ./Formula/correo.rb` か
  #           `curl -sL https://github.com/fuyutarow/correo/archive/refs/tags/v0.1.0.tar.gz | shasum -a 256`
  #           で sha256 を確定し、下の 0000… を置換する。
  url "https://github.com/fuyutarow/correo/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "0000000000000000000000000000000000000000000000000000000000000000"
  license any_of: ["MIT", "Apache-2.0"]
  head "https://github.com/fuyutarow/correo.git", branch: "main"

  depends_on "rust" => :build

  # coinage の live エンジン用 Sudachi 辞書「本体」(core・展開後 ~207MB)。RG4: sha256 実検証済 (2026-07)。
  # サイズを嫌う場合は core → small (sudachi-dictionary-20260428-small.zip) へ差し替え可 (被覆は下がる)。
  resource "sudachidict" do
    url "https://github.com/WorksApplications/SudachiDict/releases/download/v20260428/sudachi-dictionary-20260428-core.zip"
    sha256 "40c8ffc095283f07aa06cae922e7b8147bf2919ec8830567b0b3f7a7efa3239f"
  end

  # Sudachi engine の「設定」resources (char.def / unk.def / rewrite.def / sudachi.json)。辞書本体とは
  # 別配布のため必須。correo が pin する engine と同一 tag (sudachi.rs v0.6.11) から取り版ズレを防ぐ。
  resource "sudachi-resources" do
    url "https://github.com/WorksApplications/sudachi.rs/archive/refs/tags/v0.6.11.tar.gz"
    sha256 "ceee381af9045d84ec77b0823cc970006e6959d53fd0ab17db37008185d0e381"
  end

  def install
    # coinage feature を明示 (既定 default だが依存を確実に引き込む)。
    system "cargo", "install", "--features", "coinage", *std_cargo_args

    # 辞書一式を share/correo/dict へ。binary は exe 相対 (bin/../share/correo/dict) を out-of-box 解決
    # する (resolve_dict_dir)。env 無設定でも system.dic 存在で coinage が動く。
    # メタファー語彙表（data）— binary は exe 相対 (bin/../share/correo/) で解決する
    (share/"correo").install "lexicons/metaphor-lex.tsv"
    dict = share/"correo/dict"
    dict.mkpath

    resource("sudachidict").stage do
      dict.install Dir["sudachi-dictionary-*/system_core.dic"].first => "system.dic"
      dict.install Dir["sudachi-dictionary-*/LICENSE-2.0.txt"].first => "LICENSE-SudachiDict"
    end

    resource("sudachi-resources").stage do
      %w[char.def unk.def rewrite.def sudachi.json].each do |f|
        dict.install "sudachi.rs-0.6.11/resources/#{f}"
      end
    end
  end

  test do
    assert_match "correo", shell_output("#{bin}/correo --version")
    assert_path_exists share/"correo/dict/system.dic"

    # codemix は辞書不要 (純 locate)。
    assert_match "CODEMIX", pipe_output("#{bin}/correo codemix", "framework を混ぜた日本語の段落。\n")

    # coinage が同梱辞書を out-of-box で load できるか (exit 0 = 辞書解決 + sudachi.json 妥当の実証)。
    # ここが落ちれば dict bundle か sudachi.json のパス解決に問題があるということ。
    pipe_output("#{bin}/correo coinage --advisory", "構造腕を書く。\n", 0)
  end
end
