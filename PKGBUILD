pkgname=pwrate
pkgver=1.0.0
pkgrel=1
pkgdesc='sample rate chooser for pipewire'
arch=(x86_64)
url='https://github.com/pkunk/pwrate'
license=(GPL3)
depends=(gtk4)
makedepends=(cargo git)
source=("$pkgname-$pkgver.tar.gz::https://github.com/pkunk/pwrate/archive/refs/tags/$pkgver.tar.gz")
sha256sums=('8413e53743b04a917a252e5979e7ac09828dadeecf7978afcec6b7afc1296d40')

build() {
  cd $pkgname-$pkgver
  cargo build --frozen --release
}

package() {
  cd $pkgname-$pkgver
  install -Dm755 target/release/pwrate -t "$pkgdir/usr/bin"
  install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}