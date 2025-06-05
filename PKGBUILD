# Maintainer: GhostKellz <ckelley@ghostkellz.sh>
pkgname=ghostbrew
pkgver=0.2.0
pkgrel=1
pkgdesc="A fast, minimal, Rust-powered AUR helper for Arch Linux."
arch=('x86_64' 'aarch64')
url="https://github.com/ghostkellz/ghostbrew"
license=('MIT')
depends=('git')
makedepends=('rust' 'cargo' 'clang' 'llvm' 'gcc' 'base-devel')
provides=('ghostbrew')
conflicts=('ghostbrew')
source=("$pkgname-$pkgver.tar.gz::https://github.com/ghostkellz/ghostbrew/archive/refs/tags/v$pkgver.tar.gz")
b2sums=('SKIP')

prepare() {
  cd "$srcdir/$pkgname-$pkgver"
  if [[ ! -f Cargo.lock ]]; then
    echo "ERROR: Cargo.lock is missing from the source tarball!" >&2
    exit 1
  fi
  cargo clean
}

build() {
  cd "$srcdir/$pkgname-$pkgver"
  cargo clean
  cargo build --release
}

package() {
  cd "$srcdir/$pkgname-$pkgver"
  install -Dm755 target/release/$pkgname "$pkgdir/usr/bin/$pkgname"
  install -Dm644 README.md "$pkgdir/usr/share/doc/$pkgname/README.md"
  install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}
