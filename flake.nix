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

        packageNativeBuildInputs = with pkgs; [
          (rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
          gnumake
          rust-cbindgen
        ];

        cargoToml = builtins.fromTOML (builtins.readFile ./lio/Cargo.toml);
        pname = cargoToml.package.name;
        version = cargoToml.package.version;
        description = cargoToml.package.description;
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          inherit pname version;
          useNextest = true;

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = packageNativeBuildInputs;

          buildInputs = with pkgs; [ stdenv.cc.cc.lib ];

          buildPhase = ''
            runHook preBuild
            make lio-cbuild
            runHook postBuild
          '';

          installPhase = ''
            mkdir -p $out/lib $out/include $out/lib/pkgconfig
            cp lio/include/lio.h $out/include/
            cp target/release/liblio${pkgs.stdenv.hostPlatform.extensions.sharedLibrary} $out/lib/

            # Generate pkg-config file
            cat > $out/lib/pkgconfig/${pname}.pc << EOF
            prefix=$out
            libdir=$out/lib
            includedir=$out/include

            Name: ${pname}
            Description: ${description}
            Version: ${version}
            Libs: -L$out/lib -l${pname} -Wl,-rpath,$out/lib
            Libs.private: -lpthread ${if pkgs.stdenv.isLinux then "-ldl" else ""}
            Cflags: -I$out/include
            EOF
          '';

          meta = with pkgs.lib; {
            inherit description;
            license = licenses.mit;
            platforms = platforms.unix;
          };
        };

        devShells =
          let
            ciNativeBuildInputs =
              packageNativeBuildInputs
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
