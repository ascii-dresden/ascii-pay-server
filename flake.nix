{
  inputs.nixpkgs.url = github:NixOS/nixpkgs/nixos-21.11;

  inputs.naersk = {
    url = github:nix-community/naersk;
    inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, naersk, ... }@inputs:
    let
      pkgs = import nixpkgs { system = "x86_64-linux"; };
      package = pkgs.callPackage ./derivation.nix {
        src = ./.;
        naersk = naersk.lib.x86_64-linux;
      };
    in
    {
      defaultPackage."x86_64-linux" = package;

      overlay = (final: prev: {
        ascii-pay-server = package;
        ascii-pay-server-src = ./.;
      });

      hydraJobs = {
        ascii-pay-server."x86_64-linux" = package;
      };
    };
}
