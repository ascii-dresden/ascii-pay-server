{
  description = "Build ascii-pay-server";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-utils.url = "github:numtide/flake-utils";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
  };

  outputs = { self, nixpkgs, crane, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };
        lib = pkgs.lib;

        rustWithWasiTarget = pkgs.rust-bin.nightly.latest.default.override { };

        craneLib = (crane.mkLib pkgs).overrideToolchain rustWithWasiTarget;

        filter = path: _type: builtins.match ".*md$|.*sql$" path != null;
        markdownOrSQLOrCargo = path: type:
          (filter path type) || (craneLib.filterCargoSources path type);

        ascii-pay-server = craneLib.buildPackage {
          src = lib.cleanSourceWith {
            src = craneLib.path ./.;
            filter = markdownOrSQLOrCargo;
          };

          doCheck = false;

        };
      in {
        inherit ascii-pay-server;

        packages.default = ascii-pay-server;
        defaultPackage."x86_64-linux" = ascii-pay-server;
        apps.default = flake-utils.lib.mkApp { drv = "ascii-pay-server"; };
        hydraJobs = {
          ascii-pay-server."x86_64-linux" = ascii-pay-server;
        };
      });
}
