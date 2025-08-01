{ pkgs, lib, config, inputs, ... }:

{
  cachix.enable = false;

  packages = [ pkgs.git pkgs.openssl ];

  languages.rust.enable = true;
}
