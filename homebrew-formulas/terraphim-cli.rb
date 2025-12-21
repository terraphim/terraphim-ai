class TerraphimCli < Formula
  desc "CLI tool for semantic knowledge graph search with JSON output"
  homepage "https://github.com/terraphim/terraphim-ai"
  version "1.0.0"
  license "Apache-2.0"

  # NOTE: macOS and Windows users should use 'cargo install terraphim-cli'
  # Pre-built binaries are only available for Linux x86_64 in v1.0.0

  on_linux do
    url "https://github.com/terraphim/terraphim-ai/releases/download/v1.0.0/terraphim-cli-linux-x86_64"
    sha256 "c217d6dbbec60ef691bbb7220b290ee420f25e39c7fd39c62099aead9be98980"
  end

  # macOS and Windows: build from source via cargo
  on_macos do
    depends_on "rust" => :build
  end

  def install
    if OS.linux?
      bin.install "terraphim-cli-linux-x86_64" => "terraphim-cli"
    else
      # macOS/other: compile from source
      system "cargo", "install", "--root", prefix, "--path", ".", "terraphim-cli"
    end
  end

  test do
    assert_match "terraphim-cli 1.0.0", shell_output("#{bin}/terraphim-cli --version")
  end
end
