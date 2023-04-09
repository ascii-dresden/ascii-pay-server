{ craneLib, src, lib }:

craneLib.buildPackage {
  pname = "ascii-pay-server";
  version = "2.0.0";

  src = lib.cleanSourceWith {
    src = craneLib.path ./.;
    filter = path: type:
      ((path: _type: builtins.match ".*md$|.*sql$" path != null) path type) || (craneLib.filterCargoSources path type);
  };

  nativeBuildInputs = [ ];
  buildInputs = [ ];
  doCheck = false;
  meta = with lib; {
    description =
      "Rust server which handles the transactions of the ascii-pay system.";
    homepage = "https://github.com/ascii-dresden/ascii-pay-server.git";
    license = with licenses; [ mit ];
  };
}
