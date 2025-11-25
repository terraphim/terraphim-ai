class TerraphimCli < Formula
  desc "CLI tool for semantic knowledge graph search with JSON output"
  homepage "https://github.com/terraphim/terraphim-ai"
  version "1.0.0"
  license "Apache-2.0"

  if OS.mac? && Hardware::CPU.intel?
    url "https://github.com/terraphim/terraphim-ai/releases/download/v1.0.0/terraphim-cli-macos-x86_64"
    sha256 "PLACEHOLDER_MACOS_X86_64"
  elsif OS.mac? && Hardware::CPU.arm?
    url "https://github.com/terraphim/terraphim-ai/releases/download/v1.0.0/terraphim-cli-macos-aarch64"
    sha256 "PLACEHOLDER_MACOS_AARCH64"
  elsif OS.linux?
    url "https://github.com/terraphim/terraphim-ai/releases/download/v1.0.0/terraphim-cli-linux-x86_64"
    sha256 "c217d6dbbec60ef691bbb7220b290ee420f25e39c7fd39c62099aead9be98980"
  end

  def install
    bin.install "terraphim-cli-linux-x86_64" => "terraphim-cli" if OS.linux?
    bin.install "terraphim-cli-macos-x86_64" => "terraphim-cli" if OS.mac? && Hardware::CPU.intel?
    bin.install "terraphim-cli-macos-aarch64" => "terraphim-cli" if OS.mac? && Hardware::CPU.arm?
  end

  test do
    assert_match "terraphim-cli 1.0.0", shell_output("#{bin}/terraphim-cli --version")
  end
end
