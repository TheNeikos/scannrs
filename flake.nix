{
  description = "library";
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-24.11";
    flake-utils = {
      url = "github:numtide/flake-utils";
    };
    crane = {
      url = "github:ipetkov/crane";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };
  };

  outputs =
    inputs:
    inputs.flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import inputs.nixpkgs {
          inherit system;
          overlays = [ (import inputs.rust-overlay) ];
        };

        rustTarget = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        unstableRustTarget = pkgs.rust-bin.selectLatestNightlyWith (
          toolchain:
          toolchain.default.override {
            extensions = [
              "rust-src"
              "miri"
              "rustfmt"
            ];
          }
        );
        craneLib = (inputs.crane.mkLib pkgs).overrideToolchain rustTarget;
        unstableCraneLib = (inputs.crane.mkLib pkgs).overrideToolchain unstableRustTarget;

        tomlInfo = craneLib.crateNameFromCargoToml { cargoToml = ./Cargo.toml; };
        inherit (tomlInfo) version;
        src = ./.;

        rustfmt' = pkgs.writeShellScriptBin "rustfmt" ''
          exec "${unstableRustTarget}/bin/rustfmt" "$@"
        '';

        nativeBuildInputs = [
          pkgs.clang
          pkgs.pkg-config
          pkgs.rustPlatform.bindgenHook
        ];

        buildInputs = [
          pkgs.sane-backends
          pkgs.libclang
        ];

        cargoArtifacts = craneLib.buildDepsOnly {
          inherit
            src
            version
            nativeBuildInputs
            buildInputs
            ;
          cargoExtraArgs = "--all-features --all";
        };

        crate = craneLib.buildPackage {
          inherit
            cargoArtifacts
            src
            version
            nativeBuildInputs
            buildInputs
            ;
          cargoExtraArgs = "--all-features --all";
        };

      in
      rec {
        checks = {
          inherit crate;

          crate-clippy = craneLib.cargoClippy {
            inherit cargoArtifacts src;
            cargoExtraArgs = "--all --all-features";
            cargoClippyExtraArgs = "-- --deny warnings";
          };

          crate-fmt = unstableCraneLib.cargoFmt {
            inherit src;
          };
        };

        packages.crate = crate;
        packages.default = packages.crate;

        apps.crate = inputs.flake-utils.lib.mkApp {
          name = "library";
          drv = crate;
        };
        apps.default = apps.crate;

        devShells.default = devShells.crate;
        devShells.crate = pkgs.mkShell {
          buildInputs = [ pkgs.libclang ];

          inputsFrom = [ crate ];

          nativeBuildInputs = [
            rustfmt'
            rustTarget
          ];

          BINDGEN_EXTRA_CLANG_ARGS = "-I${pkgs.sane-backends}/include";
        };
      }
    );
}
