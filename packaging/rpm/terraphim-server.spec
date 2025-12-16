%global __requires_exclude %{?__requires_exclude:%__requires_exclude}%{nil}
%global __provides_exclude %{?__provides_exclude:%__provides_exclude}%{nil}

Name:           terraphim-server
Version:        1.0.0
Release:        1%{?dist}
Summary:        Terraphim AI Server for Knowledge Graph Search
License:        MIT
URL:            https://github.com/terraphim/terraphim-ai
Source0:        https://github.com/terraphim/terraphim-ai/archive/v%{version}.tar.gz

BuildArch:      x86_64
Requires:       openssl-libs >= 1.1.1
Requires:       sqlite-libs >= 3.35.0
Requires(pre):   shadow-utils
BuildRequires:  rust >= 1.70.0
BuildRequires:  cargo >= 1.70.0
BuildRequires:  openssl-devel
BuildRequires:  sqlite-devel
BuildRequires:  systemd

%description
Terraphim AI server component for knowledge graph search and AI assistance.
Provides privacy-first AI search capabilities with local processing
and semantic understanding of documents and code.

%prep
%autosetup -n terraphim-ai-%{version}

%build
# Build the Rust server
cargo build --release --package terraphim_server --target %{_target_platform}

%install
# Create directories
mkdir -p %{buildroot}%{_bindir}
mkdir -p %{buildroot}%{_unitdir}
mkdir -p %{buildroot}%{_sysconfdir}/terraphim-server
mkdir -p %{buildroot}%{_datadir}/terraphim-server
mkdir -p %{buildroot}%{_localstatedir}/lib/terraphim-server
mkdir -p %{buildroot}%{_mandir}/man1

# Install binary
install -D -m 755 target/%{_target_platform}/release/terraphim_server %{buildroot}%{_bindir}/terraphim_server

# Install systemd service
install -D -m 644 terraphim_server/terraphim-server.service %{buildroot}%{_unitdir}/terraphim-server.service

# Install default configuration
install -D -m 644 terraphim_server/default/atomic_server_config_final.json %{buildroot}%{_sysconfdir}/terraphim-server/config.json

# Install man page
install -D -m 644 terraphim_server/terraphim-server.1 %{buildroot}%{_mandir}/man1/terraphim-server.1

# Install data files
cp -r terraphim_server/default/* %{buildroot}%{_datadir}/terraphim-server/

%pre
# Create system user
getent group terraphim >/dev/null || groupadd -r terraphim
getent passwd terraphim >/dev/null || useradd -r -g terraphim -d %{_localstatedir}/lib/terraphim-server -s /sbin/nologin -c "Terraphim AI Server" terraphim

%post
# Reload systemd and enable service
systemctl daemon-reload 2>/dev/null || true
systemctl --no-reload enable terraphim-server.service 2>/dev/null || true

%preun
if [ $1 -eq 0 ]; then
    # Package removal, stop the service
    systemctl --no-reload disable terraphim-server.service 2>/dev/null || true
    systemctl --no-reload stop terraphim-server.service 2>/dev/null || true
fi

%postun
systemctl daemon-reload 2>/dev/null || true

%files
%license LICENSE*
%doc README.md*
%{_bindir}/terraphim_server
%{_unitdir}/terraphim-server.service
%config %{_sysconfdir}/terraphim-server/config.json
%{_datadir}/terraphim-server/
%{_mandir}/man1/terraphim-server.1*
%dir %{_localstatedir}/lib/terraphim-server
%ghost %{_localstatedir}/lib/terraphim-server/*

%changelog
* Sun Dec 16 2025 Terraphim Team <team@terraphim.ai> - 1.0.0-1
- Initial RPM release
- Includes systemd service and default configuration
- Supports knowledge graph search and AI assistance