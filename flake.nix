{
  description = "Rust PHP 8.5 engine development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { nixpkgs, ... }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in
    {
      devShells = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
          inherit (pkgs) lib stdenv;
          rustcSccacheWrapper = pkgs.writeShellScriptBin "rustc-sccache-wrapper" ''
            unset CARGO_INCREMENTAL
            exec ${pkgs.sccache}/bin/sccache "$@"
          '';

          commonPackages = with pkgs; [
            rustc
            cargo
            rustfmt
            clippy
            rust-analyzer

            git
            curl
            wget
            cacert
            just
            jq
            ripgrep
            fd
            tree
            shellcheck

            autoconf
            automake
            libtool
            bison
            re2c
            gnumake
            pkg-config
            ccache
            sccache

            cmake
            ninja
            clang

            libxml2
            sqlite
            openssl
            zlib
            bzip2
            xz
            libzip

            python3
          ];

          linuxPackages = with pkgs; [
            gdb
            mold
          ];

          darwinPackages = with pkgs; [
            libiconv
          ];
        in
        {
          default = pkgs.mkShell {
            packages =
              commonPackages
              ++ lib.optionals stdenv.isLinux linuxPackages
              ++ lib.optionals stdenv.isDarwin darwinPackages;

            PHP_REF_SERIES = "8.5";
            PHP_REF_VERSION = "8.5.7";
            PHP_REF_TAG = "php-8.5.7";
            PHP_REF_REPO = "https://github.com/php/php-src.git";
            RUST_BACKTRACE = "1";
            RUSTC_WRAPPER = "${rustcSccacheWrapper}/bin/rustc-sccache-wrapper";
            SCCACHE_CACHE_SIZE = "20G";
            CARGO_REGISTRIES_CRATES_IO_PROTOCOL = "sparse";

            shellHook =
              ''
                export CARGO_TARGET_DIR="$PWD/target"
                export SCCACHE_DIR="$PWD/.cache/sccache"
                export CCACHE_DIR="$PWD/.cache/ccache"
                mkdir -p "$CARGO_TARGET_DIR" "$SCCACHE_DIR" "$CCACHE_DIR"
              ''
              + lib.optionalString stdenv.isLinux ''
                export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=clang
                export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=clang
                case " ${RUSTFLAGS:-} " in
                  *" -C link-arg=-fuse-ld=mold "*) ;;
                  *) export RUSTFLAGS="-C link-arg=-fuse-ld=mold ${RUSTFLAGS:-}" ;;
                esac
              ''
              + ''
              echo "phrust dev shell" >&2
              echo "  Rust cache: $SCCACHE_DIR" >&2
              echo "  Cargo target: $CARGO_TARGET_DIR" >&2
              echo "  just help" >&2
              echo "  just verify" >&2
            '';
          };
        }
      );
    };
}
