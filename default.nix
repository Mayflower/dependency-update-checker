with import <nixpkgs> {};

rustPlatform.buildRustPackage {
  name = "duck";
  src = ./.;
  buildInputs = [
    openssl
    cmake
    zlibStatic
  ];
  depsSha256 = "0idqwqi5dnrpfpn576460a9m2l7hkbhk26kxlw9j7j7bnqrw5pdr";
}
