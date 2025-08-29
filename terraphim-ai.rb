class TerraphimAi < Formula
  desc "Privacy-first AI assistant with semantic search and knowledge graphs"
  homepage "https://github.com/terraphim/terraphim-ai"
  url "https://github.com/terraphim/terraphim-ai/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "0000000000000000000000000000000000000000000000000000000000000000" # Will need to be updated with actual SHA256
  license "MIT"
  head "https://github.com/terraphim/terraphim-ai.git", branch: "main"

  depends_on "rust" => :build
  depends_on "node" => :build
  depends_on "yarn" => :build
  depends_on "pkg-config" => :build
  depends_on "openssl@3"
  depends_on "sqlite"

  def install
    # Build the Rust components
    system "cargo", "build", "--release", "--bin", "terraphim_server"
    
    # Build the desktop app (if on macOS)
    if OS.mac?
      cd "desktop" do
        system "yarn", "install"
        system "yarn", "run", "tauri", "build"
      end
      
      # Install the desktop app bundle
      app_bundle = "target/release/bundle/macos/Terraphim Desktop.app"
      if File.exist?(app_bundle)
        prefix.install app_bundle => "Terraphim Desktop.app"
      end
    end
    
    # Install the server binary
    bin.install "target/release/terraphim_server"
    
    # Install configuration files
    (etc/"terraphim-ai").mkpath
    (etc/"terraphim-ai").install Dir["terraphim_server/default/*.json"]
    
    # Install documentation
    doc.install "README.md"
    doc.install "docs" if Dir.exist?("docs")
  end

  def caveats
    <<~EOS
      Terraphim AI has been installed with the following components:
      
      1. Server: Run with `terraphim_server`
      2. Desktop App: Available in Applications folder (macOS only)
      
      Default configuration files are located in:
        #{etc}/terraphim-ai/
      
      For first-time setup:
        1. Run `terraphim_server --help` to see available options
        2. Configuration files can be customized in #{etc}/terraphim-ai/
        3. The desktop app will create its own config on first run
      
      Documentation is available in:
        #{doc}/
    EOS
  end

  service do
    run opt_bin/"terraphim_server"
    keep_alive true
    error_log_path var/"log/terraphim-ai-error.log"
    log_path var/"log/terraphim-ai.log"
    working_dir HOMEBREW_PREFIX
  end

  test do
    # Test that the server binary was installed and shows version info
    system "#{bin}/terraphim_server", "--version"
    
    # Test that config files were installed
    assert_predicate etc/"terraphim-ai", :exist?
    
    # Test basic functionality by checking the help output
    help_output = shell_output("#{bin}/terraphim_server --help")
    assert_match "Terraphim AI Server", help_output
  end
end