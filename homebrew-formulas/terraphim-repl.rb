class TerraphimRepl < Formula
  desc "Interactive REPL for semantic knowledge graph search"
  homepage "https://github.com/terraphim/terraphim-ai"
  version "1.0.0"
  license "Apache-2.0"

  # NOTE: macOS and Windows users should use 'cargo install terraphim-repl'
  # Pre-built binaries are only available for Linux x86_64 in v1.0.0

  on_linux do
    url "https://github.com/terraphim/terraphim-ai/releases/download/v1.0.0/terraphim-repl-linux-x86_64"
    sha256 "73fa4b15aae497ad20939bc02767fec1d56583748ceef231c2bd58b4f9dc98b2"
  end

  # macOS and Windows: build from source via cargo
  on_macos do
    depends_on "rust" => :build
  end

  def install
    if OS.linux?
      bin.install "terraphim-repl-linux-x86_64" => "terraphim-repl"
    else
      # macOS/other: compile from source
      system "cargo", "install", "--root", prefix, "--path", ".", "terraphim-repl"
    end
  end

  test do
    assert_match "terraphim-repl 1.0.0", shell_output("#{bin}/terraphim-repl --version")
  end
end
