class TerraphimRepl < Formula
  desc "Interactive REPL for semantic knowledge graph search"
  homepage "https://github.com/terraphim/terraphim-ai"
  version "1.0.0"
  license "Apache-2.0"

  if OS.mac? && Hardware::CPU.intel?
    url "https://github.com/terraphim/terraphim-ai/releases/download/v1.0.0/terraphim-repl-macos-x86_64"
    sha256 "PLACEHOLDER_MACOS_X86_64"
  elsif OS.mac? && Hardware::CPU.arm?
    url "https://github.com/terraphim/terraphim-ai/releases/download/v1.0.0/terraphim-repl-macos-aarch64"
    sha256 "PLACEHOLDER_MACOS_AARCH64"
  elsif OS.linux?
    url "https://github.com/terraphim/terraphim-ai/releases/download/v1.0.0/terraphim-repl-linux-x86_64"
    sha256 "73fa4b15aae497ad20939bc02767fec1d56583748ceef231c2bd58b4f9dc98b2"
  end

  def install
    bin.install "terraphim-repl-linux-x86_64" => "terraphim-repl" if OS.linux?
    bin.install "terraphim-repl-macos-x86_64" => "terraphim-repl" if OS.mac? && Hardware::CPU.intel?
    bin.install "terraphim-repl-macos-aarch64" => "terraphim-repl" if OS.mac? && Hardware::CPU.arm?
  end

  test do
    assert_match "terraphim-repl 1.0.0", shell_output("#{bin}/terraphim-repl --version")
  end
end
