{
  inputs.nixpkgs.url = github:NixOS/nixpkgs/nixos-unstable;

  inputs.naersk = {
    url = github:nix-community/naersk;
    inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, naersk, ... }@inputs:
    let
      pkgs = import nixpkgs {
        system = "x86_64-linux";
        overlays = [ overlay ];
      };
      overlay = final: prev: {
        ascii-pay-server = final.callPackage ./derivation.nix {
          src = ./.;
          naersk = naersk.lib.${final.system};
        };
        ascii-pay-server-src = ./.;
      };
    in
    {
      defaultPackage."x86_64-linux" = pkgs.ascii-pay-server;
      inherit overlay;

      hydraJobs = {
        ascii-pay-server."x86_64-linux" = pkgs.ascii-pay-server;
        ascii-pay-server."x86_64-linux-static" = pkgs.pkgsStatic.ascii-pay-server;
      };
    };
}
