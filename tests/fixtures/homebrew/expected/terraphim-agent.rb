class TerraphimAgent < Formula
  desc "Interactive TUI and REPL for Terraphim AI semantic search"
  homepage "https://github.com/terraphim/terraphim-clients"

  target = on_system_conditional(
    macos: "universal-apple-darwin",
    linux: on_arch_conditional(
      arm:   "aarch64-unknown-linux-musl",
      intel: "x86_64-unknown-linux-gnu",
    ),
  )
  checksum = on_system_conditional(
    macos: "0a9e7eb47508d285f66966db7fd2bb36367ad847c0275a068796c7c382c107da",
    linux: on_arch_conditional(
      arm:   "f8530d963171e3f72b52b8b729653cafa5ae5e105e219f7ecec7b170037acaa7",
      intel: "b6dec5834b96fa3178680b0804f5e6ce47a2a201e7f70e1e859b67359dd00ad4",
    ),
  )
  url "https://downloads.terraphim.ai/terraphim-agent/terraphim-agent-1.21.16-#{target}.tar.gz"
  mirror "https://github.com/terraphim/terraphim-clients/releases/download/v1.21.16/terraphim-agent-1.21.16-#{target}.tar.gz"
  sha256 checksum
  license "Apache-2.0"

  def install
    bin.install "terraphim-agent"
  end

  test do
    assert_match "terraphim", shell_output("#{bin}/terraphim-agent --version 2>&1")
    assert_match "Learning capture", shell_output("#{bin}/terraphim-agent learn --help 2>&1")
    assert_match "Session management", shell_output("#{bin}/terraphim-agent sessions --help 2>&1")
    if OS.mac?
      system "/usr/bin/codesign", "--verify", "--all-architectures", "--deep", "--strict",
             bin/"terraphim-agent"
    end
  end
end
