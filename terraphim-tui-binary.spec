Name:           terraphim-tui
Version:        0.2.3
Release:        1%{?dist}
Summary:        Terraphim TUI - Terminal User Interface for Terraphim AI

License:        Apache-2.0
URL:            https://terraphim.ai

%description
Terraphim TUI - Terminal User Interface for Terraphim AI.
Command-line interface with interactive REPL and ASCII graph visualization.
Supports search, configuration management, and data exploration.

%install
mkdir -p %{buildroot}%{_bindir}
mkdir -p %{buildroot}%{_docdir}/%{name}
mkdir -p %{buildroot}%{_licensedir}/%{name}

install -m755 target/release/terraphim-tui %{buildroot}%{_bindir}/terraphim-tui
install -m644 README.md %{buildroot}%{_docdir}/%{name}/
install -m644 LICENSE-Apache-2.0 %{buildroot}%{_licensedir}/%{name}/

%files
%license LICENSE-Apache-2.0
%doc README.md
%{_bindir}/terraphim-tui

%changelog
* Mon Oct 14 2024 Terraphim Contributors <team@terraphim.ai> - 0.2.3-1
- Initial binary RPM release
- Terminal User Interface for Terraphim AI
- Interactive REPL and ASCII graph visualization