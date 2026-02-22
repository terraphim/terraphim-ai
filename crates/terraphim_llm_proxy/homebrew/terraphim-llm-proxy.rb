# Homebrew formula for Terraphim LLM Proxy
class TerraphimLlmProxy < Formula
  desc "Production-ready intelligent LLM routing proxy for Claude Code"
  homepage "https://github.com/terraphim/terraphim-llm-proxy"
  url "https://github.com/terraphim/terraphim-llm-proxy.git"
  license "MIT"
  head "https://github.com/terraphim/terraphim-llm-proxy.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: "terraphim-llm-proxy")
  end

  test do
    # Basic version test
    system "#{bin}/terraphim-llm-proxy", "--version"

    # Test with config file
    (testpath/"config.toml").write <<~TOML
      [server]
      host = "127.0.0.1"
      port = 3456

      [providers.openrouter]
      api_key = "test-key"
      base_url = "https://openrouter.ai/api/v1"
    TOML

    # Test that the proxy can parse configuration
    system "#{bin}/terraphim-llm-proxy", "--config", "#{testpath}/config.toml", "--help"
  end

  service do
    run [opt_bin/"terraphim-llm-proxy", "--config", etc/"terraphim-llm-proxy/config.toml"]
    keep_alive true
    log_path var/"log/terraphim-llm-proxy.log"
    error_log_path var/"log/terraphim-llm-proxy.error.log"
  end
end