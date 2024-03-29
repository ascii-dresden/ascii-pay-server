{
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

  outputs = inputs@{ self, nixpkgs, flake-utils, crane, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };
        lib = pkgs.lib;
        rust = pkgs.rust-bin.nightly.latest.default.override { };

        craneLib = (crane.mkLib pkgs).overrideToolchain rust;
        package = pkgs.callPackage ./derivation.nix { craneLib = craneLib; };
      in rec {
        doCheck = false;
        packages = {
          ascii-pay-server = package;
          default = package;
        };
        apps = {
          ascii-pay-server =
            flake-utils.lib.mkApp { drv = packages.ascii-pay-server; };
          default = apps.ascii-pay-server;
        };

        hydraJobs = {
          ascii-pay-server."x86_64-linux" = packages.ascii-pay-server;
        };
      }) // {
        nixosModules = rec {
          default = ascii-pay-server;
          ascii-pay-server = import ./nixos-module;
        };

        overlays.default = final: prev: {
          inherit (self.packages.${prev.system}) ascii-pay-server;
        };
      };
}
