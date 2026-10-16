{
  description = "maa-cli development environment";

  inputs.nixpkgs.url = "https://channels.nixos.org/nixpkgs-unstable/nixexprs.tar.xz";

  outputs =
    { self, nixpkgs }:
    let
      forAllSystems =
        f:
        nixpkgs.lib.genAttrs [
          "x86_64-linux"
          "aarch64-linux"
          "x86_64-darwin"
          "aarch64-darwin"
        ] (system: f nixpkgs.legacyPackages.${system});
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
            pkgs.rustup
            pkgs.pkg-config
            pkgs.openssl
          ];
        };
      });
    };
}
