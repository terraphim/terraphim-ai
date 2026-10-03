class TerraphimGrep < Formula
  desc "Intelligent hybrid grep with knowledge-graph boosting and LLM fallback"
  homepage "https://github.com/terraphim/terraphim-clients"

  target = on_system_conditional(
    macos: "universal-apple-darwin",
    linux: on_arch_conditional(
      arm:   "aarch64-unknown-linux-musl",
      intel: "x86_64-unknown-linux-gnu",
    ),
  )
  checksum = on_system_conditional(
    macos: "5c97821855ae2d6fee5df00c2baa912c0b30df60f2bb3f351baec0b0d36ed3e0",
    linux: on_arch_conditional(
      arm:   "194d4cac04eccbfb998c4f060e44b286873510d354fd3a3a160bf63b2c5e025c",
      intel: "8f396e508aeecc8743052f2ef8f37c8ec1502b4c31de1f74c1f7efdbf42b27e6",
    ),
  )
  url "https://downloads.terraphim.ai/terraphim-grep/terraphim-grep-1.21.16-#{target}.tar.gz"
  mirror "https://github.com/terraphim/terraphim-clients/releases/download/v1.21.16/terraphim-grep-1.21.16-#{target}.tar.gz"
  sha256 checksum
  license "MIT"

  def install
    bin.install "terraphim-grep"
  end

  test do
    assert_match "terraphim", shell_output("#{bin}/terraphim-grep --version 2>&1")
    assert_match "Intelligent hybrid grep", shell_output("#{bin}/terraphim-grep --help 2>&1")
    if OS.mac?
      system "/usr/bin/codesign", "--verify", "--all-architectures", "--deep", "--strict",
             bin/"terraphim-grep"
    end
  end
end
