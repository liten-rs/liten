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
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

      in
      {
        devShells =
          let
            buildInputs = with pkgs; [
              (rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
              cargo-nextest
              cargo-hack
            ];
          in
          {
            ci = pkgs.mkShell {
              inherit buildInputs;
            };
            default = pkgs.mkShell {
              buildInputs =
                buildInputs
                ++ (with pkgs; [
                  cargo-watch
                  cargo-expand
                ]);
            };
          };
      }
    );
}
