pkgname=ghostbrew
pkgver=0.2.0
pkgrel=1
pkgdesc="A fast, minimal, Rust-powered AUR helper for Arch Linux."
arch=('x86_64' 'aarch64')
url="https://github.com/ghostkellz/ghostbrew"
license=('MIT')
depends=('rust' 'git' 'base-devel' 'make' 'gcc' 'pkgconf')
makedepends=('cargo')
provides=('ghostbrew')
conflicts=('ghostbrew')
source=("$pkgname-$pkgver.tar.gz::https://github.com/ghostkellz/ghostbrew/archive/refs/tags/v$pkgver.tar.gz")
b2sums=('SKIP')

build() {
  cd "$srcdir/$pkgname-$pkgver"
  cargo build --release --locked
}

package() {
  cd "$srcdir/$pkgname-$pkgver"
  install -Dm755 target/release/ghostbrew "$pkgdir/usr/bin/ghostbrew"
  install -Dm644 README.md "$pkgdir/usr/share/doc/ghostbrew/README.md"
  install -Dm644 LICENSE "$pkgdir/usr/share/licenses/ghostbrew/LICENSE"
}
