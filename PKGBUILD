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
  export RUSTFLAGS="-C target-feature=-crt-static"
}

build() {
  cd "$srcdir/$pkgname-$pkgver"
  cargo build --release --locked
}

package() {
  cd "$srcdir/$pkgname-$pkgver"
  install -Dm755 target/release/$pkgname "$pkgdir/usr/bin/$pkgname"
  install -Dm644 README.md "$pkgdir/usr/share/doc/$pkgname/README.md"
  install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}
