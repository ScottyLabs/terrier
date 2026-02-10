{ pkgs, config, ... }:

let
  cargoNix = pkgs.callPackage ./Cargo.nix { };
  terrier = cargoNix.workspaceMembers.terrier-server.build;

  # the minio module can only use its MINIO_ROOT_USER and MINIO_ROOT_PASSWORD
  # env vars, so ensure they match our S3_ACCESS_KEY and S3_SECRET_KEY vars
  s3 = {
    accessKey = "terrier";
    secretKey = "terrieradmin";
  };
in
{
  packages = [
    terrier
  ] ++ (with pkgs; [
    # Project tooling
    dioxus-cli
    just
    bun

    # Database tooling
    sea-orm-cli
    minio-client
    postgresql_18
  ]);

  outputs = { inherit terrier; };

  env = {
    CARGO_PROFILE_DEV_DEBUG = "0";
    CARGO_PROFILE_DEV_CODEGEN_BACKEND = "cranelift";

    # TODO: build scripts use LLVM since cranelift lacks aarch64 CRC32 intrinsics
    # https://github.com/rust-lang/rustc_codegen_cranelift/issues/171
    # https://github.com/srijs/rust-crc32fast/pull/52
    CARGO_PROFILE_DEV_BUILD_OVERRIDE_CODEGEN_BACKEND = "llvm";

    DATABASE_URL = "postgres:///terrier?host=$PGHOST";
    REDIS_URL = "redis+unix://$REDIS_UNIX_SOCKET";
    S3_ENDPOINT = "http://localhost:9000";
    S3_ACCESS_KEY = s3.accessKey;
    S3_SECRET_KEY = s3.secretKey;
    S3_BUCKET = "terrier";
    HOST = "127.0.0.1";
    PORT = "3000";
    RUST_LOG = "debug";
  };

  languages.rust = {
    enable = true;
    channel = "nightly";
    components = [
      "rustc"
      "cargo"
      "clippy"
      "rustfmt"
      "rust-analyzer"
      "rust-src"
      "llvm-tools-preview"
      "rustc-codegen-cranelift-preview"
    ];
    mold.enable = pkgs.stdenv.isLinux;
    rustflags = "-Zthreads=8";
  };

  services.postgres = {
    enable = true;
    package = pkgs.postgresql_18;
    listen_addresses = "127.0.0.1";
    port = 5432;
    initialDatabases = [
      { name = "terrier"; }
    ];
    initialScript = ''
      CREATE USER terrier WITH PASSWORD 'terrier';
      GRANT ALL PRIVILEGES ON DATABASE terrier TO terrier;
      ALTER DATABASE terrier OWNER TO terrier;
    '';
  };

  services.redis = {
    enable = true;
    package = pkgs.valkey;
    port = 0; # unix socket mode
  };

  services.minio = {
    enable = true;
    accessKey = s3.accessKey;
    secretKey = s3.secretKey;
    buckets = [ "terrier" ];
  };

  processes.minio.process-compose.readiness_probe = {
    http_get = {
      host = "localhost";
      port = 9000;
      path = "/minio/health/live";
    };
    initial_delay_seconds = 0.5;
    period_seconds = 0.5;
  };

  git-hooks.hooks = {
    nixpkgs-fmt.enable = true;
    clippy = {
      enable = true;
      packageOverrides.cargo = config.languages.rust.toolchainPackage;
      packageOverrides.clippy = config.languages.rust.toolchainPackage;
    };
    rustfmt.enable = true;
    biome.enable = true;
    cargo-nix-update = {
      enable = true;
      name = "cargo-nix-update";
      entry = "${pkgs.writeShellScript "cargo-nix-update" ''
        if git diff --cached --name-only | grep -q '^Cargo\.\(toml\|lock\)'; then
          ${pkgs.crate2nix}/bin/crate2nix generate
          git add Cargo.nix
        fi
      ''}";
      files = "Cargo\\.(toml|lock)$";
      language = "system";
      pass_filenames = false;
    };
  };
}
