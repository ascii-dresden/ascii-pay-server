{ naersk, src, lib, pkg-config, protobuf, gcc, cmake, openssl, libpqxx, libiconv, postgresql, git }:

naersk.buildPackage {
  pname = "ascii-pay-server";
  version = "0.1.0";

  inherit src;

  cargoSha256 = lib.fakeSha256;

  nativeBuildInputs = [ pkg-config protobuf gcc cmake ];
  buildInputs = [ openssl libpqxx libiconv postgresql git ];

  installPhase = ''
    cp -r AsciiPayCard.pass $out/
  '';

  meta = with lib; {
    description = "Rust server which handles the transactions of the ascii-pay system.";
    homepage = "https://github.com/ascii-dresden/ascii-pay-server.git";
    license = with licenses; [ mit ];
  };
}
