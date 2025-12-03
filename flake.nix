{
  description = "A Rust project development environment using Nix flakes";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";

    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
    }:
    {
      overlays.default = final: prev: {
        lio = self.packages.${final.system}.default;
      };
    }
    // flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        baseBuildInputs = with pkgs; [
          (rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
          gnumake
          rust-cbindgen
        ];

      in
      {
        packages.default = import ./default.nix { inherit pkgs; };

        devShells =
          let
            ciNativeBuildInputs =
              baseBuildInputs
              ++ (with pkgs; [
                cargo-nextest
                cargo-hack
                gcc
              ]);
          in
          {
            ci = pkgs.mkShell {
              nativeBuildInputs = ciNativeBuildInputs;
            };
            default = pkgs.mkShell {
              buildInputs =
                ciNativeBuildInputs
                ++ (with pkgs; [
                  cargo-expand
                ]);
            };
          };
      }
    );
}
