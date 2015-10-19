with import <nixpkgs> {};

rustPlatform.buildRustPackage {
  name = "duck";
  src = ./.;
  buildInputs = [
    libressl
    cmake
    zlibStatic
  ];
  depsSha256 = "1idmd0n5qazhvczd5aw3jkgkz5plg6r7fkaljpc9017qy5ddkf99";
}
