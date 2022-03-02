{
  inputs.nixpkgs.url = github:NixOS/nixpkgs/nixos-21.11;

  inputs.naersk = {
    url = github:nix-community/naersk;
    inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, naersk, ... }@inputs: 
  let
    pkgs = import nixpkgs { system = "x86_64-linux"; };
  in {
    defaultPackage."x86_64-linux" = pkgs.callPackage ./derivation.nix {
      src = ./.;
      naersk = naersk.lib.x86_64-linux;
    };

    overlay = (final: prev: {
      ascii-pay-server = pkgs.callPackage ./derivation.nix {
        src = ./.;
        naersk = naersk.lib.x86_64-linux;
      };
      ascii-pay-server-src = ./.;
    });
  };
}
