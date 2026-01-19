{
  description = "tau-lang flake";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
    pre-commit = {
      url = "github:cachix/pre-commit-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs = inputs @ {
    self,
    flake-parts,
    nixpkgs,
    rust-overlay,
    ...
  }:
    flake-parts.lib.mkFlake {inherit inputs;} {
      imports = [inputs.pre-commit.flakeModule];
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      perSystem = {
        config,
        self',
        inputs',
        system,
        ...
      }: let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [(import rust-overlay)];
        };
        bi = with pkgs; [
          openssl
          pkg-config
          rust-bin.stable.latest.default
          rust-analyzer
        ];
        devDependencies = with pkgs; [
          bacon
          sccache
          perf
          nixfmt-rfc-style
        ];
      in
        with pkgs; {
          pre-commit.settings.hooks = {
            alejandra.enable = true;
            rustfmt.enable = true;
          };
          devShells.default = pkgs.mkShell {
            buildInputs = bi ++ devDependencies;
            shellHook = ''
              ${config.pre-commit.installationScript}
              alias perf-record="perf record --call-graph dwarf -- ./target/debug/tau examples/vec2.tau"
              alias perf-report="perf script report gecko"
            '';
          };
          packages = rec {
            default = tau;

            tau = rustPlatform.buildRustPackage {
              pname = "tau";
              version = "0.1.0";
              src = ./.;
              cargoLock.lockFile = ./Cargo.lock;
              checkType = "debug";
              nativeBuildInputs = [pkgs.pkg-config];
              buildInputs = bi;
            };

            tau-manpages = pkgs.stdenv.mkDerivation {
              pname = "tau-manpages";
              version = "0.1.0";

              src = ./.;

              installPhase = ''
                mkdir -p $out/share/man/man1
                cp man/tau.1 $out/share/man/man1/
              '';

              meta = {
                homepage = "https://github.com/tau-lang/ochtendzon";
                license = lib.licenses.eupl12;
                platforms = lib.platforms.all;
              };
            };
          };
        };
    };
}
