Name:           terraphim-server
Version:        0.2.3
Release:        1%{?dist}
Summary:        Terraphim AI Server - Privacy-first AI assistant backend

License:        Apache-2.0
URL:            https://terraphim.ai
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  gcc rust cargo openssl-devel

%description
Terraphim AI Server - Privacy-first AI assistant backend.
Provides HTTP API for semantic search and knowledge graphs.
Operates locally with support for multiple knowledge repositories.

%prep
%autosetup

%build
export RUSTFLAGS="-C link-arg=-fuse-ld=lld"
cargo build --release --package terraphim_server

%install
install -Dm755 target/release/terraphim_server %{buildroot}%{_bindir}/terraphim_server
install -d %{buildroot}%{_sysconfdir}/terraphim-ai
install -m644 terraphim_server/default/*.json %{buildroot}%{_sysconfdir}/terraphim-ai/
install -d %{buildroot}%{_docdir}/%{name}
install -m644 README.md %{buildroot}%{_docdir}/%{name}/
install -d %{buildroot}%{_licensedir}/%{name}
install -m644 LICENSE-Apache-2.0 %{buildroot}%{_licensedir}/%{name}/

%files
%license LICENSE-Apache-2.0
%doc README.md
%{_bindir}/terraphim_server
%dir %{_sysconfdir}/terraphim-ai
%config %{_sysconfdir}/terraphim-ai/*.json

%changelog
* Mon Oct 14 2024 Terraphim Contributors <team@terraphim.ai> - 0.2.3-1
- Initial RPM release
- Privacy-first AI assistant backend
- HTTP API for semantic search and knowledge graphs