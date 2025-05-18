# Maintainer: Christopher Kelley <ckelley@ghostkellz.sh>

pkgname=ghostbrew
gitname=ghostbrew
gituser=ghostkellz
pkgver=1.0.0
pkgrel=1
pkgdesc="Fast, minimal, and security-focused AUR helper (yay/paru replacement)"
arch=('x86_64')
url="https://github.com/ghostkellz/ghostbrew"
license=('MIT')
depends=('go' 'git' 'makepkg' 'sudo')
makedepends=('go' 'git')
provides=('ghostbrew')
conflicts=('ghostbrew')
source=("git+https://github.com/${gituser}/${gitname}.git")
md5sums=('SKIP')

pkgver() {
  cd "$srcdir/$gitname"
  git describe --tags --abbrev=0 | sed 's/^v//;s/-/./g'
}

build() {
  cd "$srcdir/$gitname"
  go build -o ghostbrew
}

package() {
  cd "$srcdir/$gitname"
  install -Dm755 ghostbrew "$pkgdir/usr/bin/ghostbrew"
  install -Dm644 README.md "$pkgdir/usr/share/doc/ghostbrew/README.md"
  install -Dm644 LICENSE "$pkgdir/usr/share/licenses/ghostbrew/LICENSE"
}
