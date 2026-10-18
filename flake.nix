{
  description = "maa-cli development environment";

  inputs = {
    nixpkgs.url = "https://channels.nixos.org/nixpkgs-unstable/nixexprs.tar.xz";
    rust-overlay = {
      url = "git+https://github.com/oxalica/rust-overlay?shallow=1";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
    }:
    let
      forAllSystems =
        f:
        nixpkgs.lib.genAttrs [
          "x86_64-linux"
          "aarch64-linux"
          "x86_64-darwin"
          "aarch64-darwin"
        ] (system: f (nixpkgs.legacyPackages.${system}.extend rust-overlay.overlays.default));
    in
    {
      # Development tools, also installable on their own with `nix profile install`.
      packages = forAllSystems (pkgs: {
        default = pkgs.buildEnv {
          name = "maa-cli-dev-tools";
          paths = with pkgs; [
            cargo-deny
            cargo-llvm-cov
            typos
            markdownlint-cli2
          ];
        };
      });

      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          packages = [
            self.packages.${pkgs.stdenv.hostPlatform.system}.default
            (pkgs.rust-bin.stable.latest.minimal.override {
              extensions = [
                "clippy"
                "llvm-tools-preview"
              ];
            })
            # rustfmt.toml uses unstable options.
            (pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.rustfmt))
            pkgs.pkg-config
            pkgs.openssl
          ];
          # There is no rustup toolchain for `cargo +nightly llvm-cov`.
          MAA_LLVM_COV_NIGHTLY = "false";
        };
      });
    };
}
