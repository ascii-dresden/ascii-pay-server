{ craneLib, src, lib, openssl, pkgconf }:

(craneLib.buildPackage {
  pname = "ascii-pay-server";
  version = "2.0.0";

  src = lib.cleanSourceWith {
    src = craneLib.path ./.;
    filter = path: type:
      ((path: _type: builtins.match ".*md$|.*sql$|.*/AsciiPayCard\\.pass(/.*|$)" path != null) path type) || (craneLib.filterCargoSources path type);
  };

  nativeBuildInputs = [ pkgconf ];
  buildInputs = [ openssl ];
  doCheck = false;
  meta = with lib; {
    description =
      "Rust server which handles the transactions of the ascii-pay system.";
    homepage = "https://github.com/ascii-dresden/ascii-pay-server.git";
    license = with licenses; [ mit ];
  };
})

# we use override attrs because setting this on the derivation itself
# affects both the final derivation and the -deps derivation,
# but $src/AsciiPayCard only exists in the final derivation
.overrideAttrs (final: prev: {
  postInstall = ''
    mkdir -p $out/share/ascii-pay-server
    cp -r $src/AsciiPayCard.pass $out/share/ascii-pay-server/
    chmod -R a+w $out/share/ascii-pay-server
  '';
})
