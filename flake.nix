{
  description = "Build the Cargo workspace for WeekendSlicer";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane.url = "github:ipetkov/crane";

    flake-parts.url = "github:hercules-ci/flake-parts";

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs =
    inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];

      perSystem =
        {
          config,
          self',
          inputs',
          pkgs,
          system,
          ...
        }:
        let
          inherit (pkgs) lib;

          craneLib = inputs.crane.mkLib pkgs;
          src = craneLib.cleanCargoSource ./.;

          # Common arguments can be set here to avoid repeating them later
          commonArgs = {
            inherit src;
            strictDeps = true;

            nativeBuildInputs = [
              pkgs.pkg-config
              pkgs.makeWrapper
            ];

            buildInputs = [
              pkgs.openssl
            ]
            ++ lib.optionals pkgs.stdenv.isDarwin [
              # Additional darwin specific inputs can be set here
              pkgs.libiconv
            ];

            # Additional environment variables can be set directly
            # MY_CUSTOM_VAR = "some value";
          };

          # Build *just* the cargo dependencies (of the entire workspace),
          # so we can reuse all of that work (e.g. via cachix) when running in CI
          # It is *highly* recommended to use something like cargo-hakari to avoid
          # cache misses when building individual top-level-crates
          cargoArtifacts = craneLib.buildDepsOnly commonArgs;

          individualCrateArgs = commonArgs // {
            inherit cargoArtifacts;
            inherit (craneLib.crateNameFromCargoToml { inherit src; }) version;
            # NB: we disable tests since we'll run them all via cargo-nextest
            doCheck = false;
          };

          fileSetForCrate =
            crate:
            lib.fileset.toSource {
              root = ./.;
              fileset = lib.fileset.unions [
                ./Cargo.toml
                ./Cargo.lock
                (craneLib.fileset.commonCargoSources ./crates/weekendslicer)
                (craneLib.fileset.commonCargoSources ./crates/workspace-hack)
                (craneLib.fileset.commonCargoSources crate)
              ];
            };

          # Build the top-level crates of the workspace as individual derivations.
          # This allows consumers to only depend on (and build) only what they need.
          # Though it is possible to build the entire workspace as a single derivation,
          # so this is left up to you on how to organize things
          #
          # Note that the cargo workspace must define `workspace.members` using wildcards,
          # otherwise, omitting a crate (like we do below) will result in errors since
          # cargo won't be able to find the sources for all members.
          weekendslicer = craneLib.buildPackage (
            individualCrateArgs
            // {
              pname = "weekendslicer";
              cargoExtraArgs = "-p weekendslicer";
              src = fileSetForCrate ./crates/weekendslicer;

              postInstall = ''
                wrapProgram $out/bin/weekendslicer \
                  --prefix LD_LIBRARY_PATH : ${
                    lib.makeLibraryPath [
                      pkgs.vulkan-loader
                      pkgs.libGL
                      pkgs.wayland
                      pkgs.libxkbcommon
                    ]
                  }
              '';
            }
          );
        in
        {
          checks = {
            # Build the crates as part of `nix flake check` for convenience
            inherit weekendslicer;

            # Run clippy (and deny all warnings) on the workspace source,
            # again, reusing the dependency artifacts from above.
            #
            # Note that this is done as a separate derivation so that
            # we can block the CI if there are issues here, but not
            # prevent downstream consumers from building our crate by itself.
            weekendslicer-workspace-clippy = craneLib.cargoClippy (
              commonArgs
              // {
                inherit cargoArtifacts;
                cargoClippyExtraArgs = "--all-targets -- --deny warnings";
              }
            );

            weekendslicer-workspace-doc = craneLib.cargoDoc (
              commonArgs
              // {
                inherit cargoArtifacts;
              }
            );

            # Check formatting
            weekendslicer-workspace-fmt = craneLib.cargoFmt {
              inherit src;
            };

            weekendslicer-workspace-toml-fmt = craneLib.taploFmt {
              src = pkgs.lib.sources.sourceFilesBySuffices src [ ".toml" ];
              # taplo arguments can be further customized below as needed
              # taploExtraArgs = "--config ./taplo.toml";
            };

            # Audit dependencies
            weekendslicer-workspace-audit = craneLib.cargoAudit {
              inherit src;
              advisory-db = inputs.advisory-db;
            };

            # Audit licenses
            weekendslicer-workspace-deny = craneLib.cargoDeny {
              inherit src;
            };

            # Run tests with cargo-nextest
            # Consider setting `doCheck = false` on other crate derivations
            # if you do not want the tests to run twice
            weekendslicer-workspace-nextest = craneLib.cargoNextest (
              commonArgs
              // {
                inherit cargoArtifacts;
                partitions = 1;
                partitionType = "count";
                cargoNextestPartitionsExtraArgs = "--no-tests=pass";
              }
            );

            # Ensure that cargo-hakari is up to date
            weekendslicer-workspace-hakari = craneLib.mkCargoDerivation {
              inherit src;
              pname = "weekendslicer-workspace-hakari";
              cargoArtifacts = null;
              doInstallCargoArtifacts = false;

              buildPhaseCargoCommand = ''
                cargo hakari generate --diff  # workspace-hack Cargo.toml is up-to-date
                cargo hakari manage-deps --dry-run  # all workspace crates depend on workspace-hack
                cargo hakari verify
              '';

              nativeBuildInputs = [
                pkgs.cargo-hakari
              ];
            };
          };

          packages = {
            inherit weekendslicer;
          };

          apps = {
            weekendslicer = {
              type = "app";
              program = "${weekendslicer}/bin/weekendslicer";
            };
          };

          devShells.default = craneLib.devShell {
            # Inherit inputs from checks.
            checks = config.checks;

            # Extra inputs can be added here; cargo and rustc are provided by default.
            packages = [
              pkgs.cargo-hakari
            ];
          };
        };
    };
}
