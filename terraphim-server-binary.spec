Name:           terraphim-server
Version:        0.2.3
Release:        1%{?dist}
Summary:        Terraphim AI Server - Privacy-first AI assistant backend

License:        Apache-2.0
URL:            https://terraphim.ai

%description
Terraphim AI Server - Privacy-first AI assistant backend.
Provides HTTP API for semantic search and knowledge graphs.
Operates locally with support for multiple knowledge repositories.

%install
mkdir -p %{buildroot}%{_bindir}
mkdir -p %{buildroot}%{_sysconfdir}/terraphim-ai
mkdir -p %{buildroot}%{_docdir}/%{name}
mkdir -p %{buildroot}%{_licensedir}/%{name}

install -m755 target/release/terraphim_server %{buildroot}%{_bindir}/terraphim_server
install -m644 terraphim_server/default/*.json %{buildroot}%{_sysconfdir}/terraphim-ai/
install -m644 README.md %{buildroot}%{_docdir}/%{name}/
install -m644 LICENSE-Apache-2.0 %{buildroot}%{_licensedir}/%{name}/

%files
%license LICENSE-Apache-2.0
%doc README.md
%{_bindir}/terraphim_server
%dir %{_sysconfdir}/terraphim-ai
%config %{_sysconfdir}/terraphim-ai/*.json

%changelog
* Mon Oct 14 2024 Terraphim Contributors <team@terraphim.ai> - 0.2.3-1
- Initial binary RPM release
- Privacy-first AI assistant backend
- HTTP API for semantic search and knowledge graphs