{
  description = "A very basic flake";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      fenix,
    }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
      fenixPkgs = fenix.packages.${system};
      rustToolChain = fenixPkgs.combine [
        fenixPkgs.stable.minimalToolchain
        fenixPkgs.stable.rustfmt
        fenixPkgs.stable.clippy
        fenixPkgs.stable.rust-src
      ];
    in
    {
      packages.${system}.default = pkgs.rustPlatform.buildRustPackage {
        pname = "scoreman";
        version = "1.0.0";
        cargoHash = "sha256-Aa92RjvcqoCCIOXoQ1TsIou/0FbLtNyb2OaaJRa90gM=";
        src = ./.;
        doCheck=false;
      };
      devShells.${system}.default = pkgs.mkShell {
        shellHook = ''
          # https://github.com/NixOS/nix/issues/8034#issuecomment-2046069655
          FLAKE_ROOT="$(git rev-parse --show-toplevel)"
          rm -f $FLAKE_ROOT/.rust-toolchain && ln -s ${rustToolChain} $FLAKE_ROOT/.rust-toolchain
        '';
        nativeBuildInputs = [
          pkgs.samply
          pkgs.valgrind
          pkgs.mold-wrapped
          pkgs.kcachegrind
          rustToolChain
        ];
      };

    };
}
