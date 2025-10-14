Name:           terraphim-tui
Version:        0.2.3
Release:        1%{?dist}
Summary:        Terraphim TUI - Terminal User Interface for Terraphim AI

License:        Apache-2.0
URL:            https://terraphim.ai
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  gcc rust cargo openssl-devel

%description
Terraphim TUI - Terminal User Interface for Terraphim AI.
Command-line interface with interactive REPL and ASCII graph visualization.
Supports search, configuration management, and data exploration.

%prep
%autosetup

%build
export RUSTFLAGS="-C link-arg=-fuse-ld=lld"
cargo build --release --package terraphim_tui --features repl-full

%install
install -Dm755 target/release/terraphim-tui %{buildroot}%{_bindir}/terraphim-tui
install -d %{buildroot}%{_docdir}/%{name}
install -m644 README.md %{buildroot}%{_docdir}/%{name}/
install -d %{buildroot}%{_licensedir}/%{name}
install -m644 LICENSE-Apache-2.0 %{buildroot}%{_licensedir}/%{name}/

%files
%license LICENSE-Apache-2.0
%doc README.md
%{_bindir}/terraphim-tui

%changelog
* Mon Oct 14 2024 Terraphim Contributors <team@terraphim.ai> - 0.2.3-1
- Initial RPM release
- Terminal User Interface for Terraphim AI
- Interactive REPL and ASCII graph visualization