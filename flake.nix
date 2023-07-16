{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    cargo2nix = {
      url = "github:cargo2nix/cargo2nix/f3f155be6487d2746fb0d5dd4e5b9d58f1658cbd";
      inputs = {
        flake-utils.follows = "flake-utils";
        nixpkgs.follows = "nixpkgs";
        rust-overlay.follows = "rust-overlay";
      };
    };
  };
  outputs = { self, flake-utils, nixpkgs, rust-overlay, cargo2nix }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) cargo2nix.overlays.default ];
        pkgs = import nixpkgs {
          inherit overlays system;
        };
        rustPlatform = pkgs.makeRustPlatform {
          cargo = pkgs.rust-bin.stable.latest.minimal;
          rustc = pkgs.rust-bin.stable.latest.minimal;
        };
        baseDependencies = with pkgs; [
          openssl
          pkg-config
          protobuf
          sqlite
          zlib
        ];
        cargoConfig = builtins.fromTOML (builtins.readFile ./.cargo/config.toml);  # TODO: Set the target CPU conditionally
        cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
        src = pkgs.lib.cleanSourceWith {
          src = pkgs.lib.cleanSource ./.;
          filter = name: type:
            let baseName = baseNameOf (toString name);
            in !(baseName == "flake.lock" || pkgs.lib.hasSuffix ".nix" baseName);
        };
        version = cargoToml.workspace.package.version;
        basePackage = {
          inherit version src;

          meta = {
            description = "ActivityPub-federated microblogging";
            homepage = "https://joinkitsune.org";
          };

          cargoLock = {
            lockFile = ./Cargo.lock;
            allowBuiltinFetchGit = true;
          };

          nativeBuildInputs = baseDependencies;

          PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig"; # Not sure why this is broken but it is
          RUSTFLAGS = builtins.concatStringsSep " " cargoConfig.build.rustflags; # Oh god help.

          checkFlags = [
            # Depend on creating an HTTP client and that reads from the systems truststore
            # Because nix is fully isolated, these types of tests fail
            #
            # Some (most?) of these also depend on the network? Not good??
            "--skip=activitypub::fetcher::test::federation_allow"
            "--skip=activitypub::fetcher::test::federation_deny"
            "--skip=activitypub::fetcher::test::fetch_actor"
            "--skip=activitypub::fetcher::test::fetch_note"
            "--skip=resolve::post::test::parse_mentions"
            "--skip=webfinger::test::fetch_qarnax_ap_id"
            "--skip=basic_request"
            "--skip=json_request"
          ];
        };
        rustPkgs = pkgs.rustBuilder.makePackageSet {
          rustVersion = "latest";
          packageFun = import ./Cargo.nix;
          workspaceSrc = src;
          rootFeatures = [ "kitsune/meilisearch" ];
        };
      in
      {
        packages = rec {
          default = main;
          cli = (rustPkgs.workspace.kitsune-cli {}).bin;
          main = (rustPkgs.workspace.kitsune { }).bin;
          search = (rustPkgs.workspace.kitsune-search {}).bin;
          frontend = pkgs.mkYarnPackage {
            inherit version;

            src = "${src}/kitsune-fe";

            buildPhase = ''
              yarn --offline build
            '';

            installPhase = ''
              mkdir -p $out
              cp -R deps/kitsune-fe/dist $out
            '';

            distPhase = "true";
          };
        };
        devShells = rec {
          default = backend;
          backend = pkgs.mkShell {
            buildInputs = with pkgs; [
              cargo-insta
              dhall
              diesel-cli
              redis
              rust-bin.stable.latest.default
            ]
            ++
            baseDependencies;
          };
          frontend = pkgs.mkShell {
            buildInputs = with pkgs; [
              nodejs
              yarn
            ];
          };
        };
      }
    ) // {
      overlays = rec {
        default = kitsune;
        kitsune = (import ./overlay.nix self);
      };

      nixosModules = rec {
        default = kitsune;
        kitsune = (import ./module.nix);
      };
    };
}
