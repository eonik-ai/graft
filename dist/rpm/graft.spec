Name:           graft
Version:        0.2.2
Release:        1%{?dist}
Summary:        Change the hook. Keep the body.

License:        Apache-2.0
URL:            https://github.com/eonik-ai/graft
Source0:        https://github.com/eonik-ai/graft/archive/refs/tags/v%{version}.tar.gz

BuildRequires:  cargo
BuildRequires:  rust
Requires:       ffmpeg

%description
Local-first composition workspace. Incremental compiler for video.
Git owns recipe history. graft owns scions, layers, and compile.

%prep
%autosetup -n graft-%{version}

%build
cargo build --release --locked --bin graft

%install
install -D -m 0755 target/release/graft %{buildroot}%{_bindir}/graft

%files
%license LICENSE
%doc README.md
%{_bindir}/graft

%changelog
* Wed Sep 16 2026 eonik <connect@eonik.ai> - 0.2.2-1
- Packager recipe for Fedora / EPEL / RHEL clones / openSUSE.
