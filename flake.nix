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
            '';
          };
          packages = {
            default = rustPlatform.buildRustPackage rec {
              pname = "tau-lang";
              version = "0.1.0";
              src = ./.;
              cargoLock.lockFile = ./Cargo.lock;
              # cargoBuildFlags = ["--package ${pname}"];
              checkType = "debug";
              nativeBuildInputs = [pkgs.pkg-config];
              buildInputs = bi;
            };
          };
        };
    };
}
