with import <nixpkgs> {};

stdenv.mkDerivation {
  name = "duck";
  buildInputs = [
    cargoSnapshot.cargo
    rustPlatform.rustc
    openssl
    cmake
    #libssh2
    #libgit2
    #pkgconfig
    zlibStatic
  ];
}
