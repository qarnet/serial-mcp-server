# shell.nix — defines the development environment for karnetvr-infra.
# When entered (via `nix-shell` or direnv), this provides Ansible
# and its dependencies, isolated from the system.

{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  name = "serial-mcp-server";

  # Tools available inside the shell.
  buildInputs = with pkgs; [
    rustc    
    cargo
    python3
    python3Packages.docker
  ];

  # Things to run when entering the shell. Useful for friendly banners
  # and environment variables that should always be set.
  shellHook = ''
    echo -e "Project: serial-mcp-server"
  '';
}
