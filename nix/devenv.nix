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
    toolchainFile = ../rust-toolchain.toml; # install targets
  };

  # Android setup
  android = {
    enable = true;
    platforms.version = [
      "33"
      "34"
    ];
    buildTools.version = [
      "33.0.0"
      "34.0.0"
    ];
    systemImageTypes = [ "google_apis" ];
  };

  # iOS setup
  apple.sdk = null; # use the system SDK

  env = {
    # Use system clang to avoid Nix wrapper's macOS flags
    CC_aarch64_apple_ios = "/usr/bin/clang";
    CXX_aarch64_apple_ios = "/usr/bin/clang++";
    CC_aarch64_apple_ios_sim = "/usr/bin/clang";
    CXX_aarch64_apple_ios_sim = "/usr/bin/clang++";

    # Use system clang for cargo linking
    CARGO_TARGET_AARCH64_APPLE_IOS_LINKER = "/usr/bin/clang";
    CARGO_TARGET_AARCH64_APPLE_IOS_SIM_LINKER = "/usr/bin/clang";
  };

  # Services
  services.postgres = {
    enable = true;
    package = pkgs.postgresql_17;
    listen_addresses = "localhost";
    port = 5432;
    initialDatabases = [ { name = "terrier"; } ];
    initialScript = ''
      CREATE USER terrier WITH PASSWORD 'terrier';
      GRANT ALL PRIVILEGES ON DATABASE terrier TO terrier;
      ALTER DATABASE terrier OWNER TO terrier;
    '';
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
    # Create Android emulator
    if ! avdmanager list avd 2>/dev/null | grep -q "Name: pixel"; then
      echo "Creating Android emulator..."
      case "$(uname -m)" in
        arm64|aarch64) abi="arm64-v8a" ;;
        *) abi="x86_64" ;;
      esac
      pkg="system-images;android-34;google_apis;$abi"
      echo "no" | avdmanager create avd --force --name pixel --package "$pkg" --device "pixel_6"
    fi

    # Create iOS simulator
    iphone_count=$(xcrun simctl list devices available 2>/dev/null | grep -c "iPhone" || echo 0)
    has_target=$(xcrun simctl list devices available 2>/dev/null | grep -c "iPhone 17 Pro" || echo 0)
    if [ "$iphone_count" -ne 1 ] || [ "$has_target" -ne 1 ]; then
      echo "Setting up iOS simulator..."
      xcrun simctl delete all 2>/dev/null || true
      xcrun simctl create "iPhone 17 Pro" "iPhone 17 Pro"
    fi

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
