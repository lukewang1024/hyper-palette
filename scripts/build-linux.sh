#!/bin/sh
# Run only inside the disposable AlmaLinux 8 release-build container.
set -eu
if [ ! -f /.dockerenv ]; then
  printf '%s\n' 'Refusing to install build dependencies outside a Docker container.' >&2
  exit 1
fi
dnf install -y dnf-plugins-core
dnf config-manager --set-enabled powertools
dnf install -y gcc gcc-c++ make git curl nasm pkgconf-pkg-config \
  fontconfig-devel freetype-devel libxcb-devel libX11-devel \
  libXcursor-devel libXi-devel libXrandr-devel libxkbcommon-devel \
  libxkbcommon-x11-devel mesa-libEGL-devel openssl-devel
export RUSTUP_HOME=/tmp/hyper-palette-rustup
export CARGO_HOME=/tmp/hyper-palette-cargo
export PATH="$CARGO_HOME/bin:$PATH"
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o /tmp/hyper-palette-rustup.sh
sh /tmp/hyper-palette-rustup.sh -y --profile minimal --default-toolchain 1.98.0
cargo test --locked
cargo build --release --locked --target x86_64-unknown-linux-gnu
