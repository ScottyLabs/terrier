{ pkgs, config, ... }:

{
  name = "terrier";

  packages = with pkgs; [
    # Build dependencies
    pkg-config
    openssl
    wasm-bindgen-cli_0_2_105 # pinned in Cargo.lock

    # Tooling
    dioxus-cli
    just

    # Database tooling
    sea-orm-cli
    minio-client
  ];

  languages.rust = {
    enable = true;
    toolchainFile = ../rust-toolchain.toml;
  };

  services.postgres = {
    enable = true;
    package = pkgs.postgresql_17;
    initialDatabases = [{ name = "terrier"; }];
  };

  services.minio = {
    enable = true;
    accessKey = builtins.getEnv "MINIO_ROOT_USER";
    secretKey = builtins.getEnv "MINIO_ROOT_PASSWORD";
    buckets = [ "terrier" ];
  };

  services.redis = {
    enable = true;
    package = pkgs.valkey;
    port = 0; # unix socket mode
  };

  git-hooks.hooks = {
    rustfmt.enable = true;
    clippy.enable = true;
    nixfmt.enable = true;
  };

  processes.postgres.process-compose.disabled = true;
  processes.redis.process-compose.disabled = true;

  processes.minio.process-compose = {
    disabled = true;
    readiness_probe = {
      http_get = {
        host = "localhost";
        port = 9000;
        path = "/minio/health/live";
      };
      initial_delay_seconds = 0.5;
      period_seconds = 0.5;
    };
  };

  enterShell = ''
    export DATABASE_URL="postgres:///terrier?host=$PGHOST"
    export REDIS_URL="redis+unix://$REDIS_UNIX_SOCKET"

    echo ""
    echo "Services:"
    echo "  PostgreSQL: socket at $PGHOST"
    echo "  Valkey:     socket at $REDIS_UNIX_SOCKET"
    echo "  MinIO:      http://localhost:9000"
    echo "  Terrier:    http://localhost:8080"
    echo ""
    echo "Use 'just' to see available commands"
    echo ""
  '';
}
