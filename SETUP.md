# Setup

This is for people who want to deploy Terrier in production.

## Deployment Options

Choose one of the following deployment methods:

- [Docker Compose](#docker-compose) - Containerized deployment with all services included
- [NixOS Module](#nixos-module) - Native deployment on NixOS with system-managed services

---

## Docker Compose

### 1. Create Environment File

Copy the example and configure for your environment:

```bash
cp examples/.env.prod .env
```

1. Update the `APP_BASE_URL` and `MINIO_PUBLIC_ENDPOINT` values to match your deployment domain.

2. Delete the "Unix sockets" section for the Redis and PostgreSQL configurations.

3. Change the `POSTGRES_PASSWORD` and `MINIO_ROOT_PASSWORD` to secure password values:

    ```bash
    openssl rand -base64 24
    ```

4. Update the "OIDC Configuration" section to match your Identity Provider.

5. Add your Global Admins' Andrew emails to `ADMIN_EMAILS`.

6. Customize the host port mapping if necessary.

### 2. Create Docker Compose File

Copy the example:

```bash
cp examples/docker-compose.yml .
```

Remove the ports on the `postgres` and `valkey` services unless you explicitly need to access them from outside the Docker network.

### 3. Start Services

```bash
docker compose up -d
```

This command will start all the services defined in your `docker-compose.yml` file in detached mode. You can check the status of the services with:

```bash
docker compose ps
```

### 4. Configure Reverse Proxy

See [Reverse Proxy Configuration](#reverse-proxy-configuration) below.

---

## NixOS Module

Native deployment using the included NixOS module with system-managed services.

### 1. Add Terrier to Your Flake

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
    terrier.url = "github:ScottyLabs/terrier";
  };

  outputs = { self, nixpkgs, terrier, ... }: {
    nixosConfigurations.your-server = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        terrier.nixosModules.default
        ./configuration.nix
      ];
    };
  };
}
```

### 2. Configure Services

In your `configuration.nix`:

```nix
{ config, pkgs, ... }:

{
  # PostgreSQL with Unix socket authentication
  services.postgresql = {
    enable = true;
    package = pkgs.postgresql_17;
    # Terrier module creates user and database
  };

  # Redis/Valkey with Unix socket
  services.redis.servers.terrier = {
    enable = true;
    port = 0;  # Unix socket only
    user = "redis-terrier";
    unixSocket = "/run/redis-terrier/redis.sock";
    unixSocketPerm = 660;
  };

  # MinIO for object storage
  services.minio = {
    enable = true;
    rootCredentialsFile = "/run/secrets/minio-credentials";
  };

  # Terrier application
  services.terrier = {
    enable = true;
    environmentFile = "/run/secrets/terrier-env";

    # Grant access to Redis socket
    extraGroups = [ "redis-terrier" ];

    # Wait for services to be ready
    dependencies = [
      "postgresql.service"
      "redis-terrier.service"
      "minio.service"
    ];
  };

  # Reverse proxy
  services.nginx = {
    enable = true;
    virtualHosts."terrier.example.com" = {
      forceSSL = true;
      enableACME = true;
      locations."/" = {
        proxyPass = "http://127.0.0.1:3000";
        proxyWebsockets = true;
      };
    };
  };
}
```

Then create the secrets files modeled after `examples/.env.prod`. Remove the "TCP via Docker internal network" and "Optional host port mapping" sections, since those are meant for Docker Compose deployments.

Customize the passwords and other fields as described in the [Docker Compose section](#1-create-environment-file). Use [sops-nix](https://github.com/Mic92/sops-nix) or [agenix](https://github.com/ryantm/agenix) to manage secrets securely.

### 4. Deploy

```bash
nixos-rebuild switch
```

---

## Connection Methods

Terrier supports both Unix socket and TCP connections for PostgreSQL and Redis.

### PostgreSQL

| Method      | `DATABASE_URL` Format                      | Use Case                |
|-------------|--------------------------------------------|-------------------------|
| Unix Socket | `postgres:///terrier?host=/run/postgresql` | NixOS, local deployment |
| TCP         | `postgres://user:pass@host:5432/terrier`   | Docker, remote database |

Unix socket authentication is more secure (no passwords or network exposure) and has slightly lower latency, so it is preferred if you can use it.

### Redis

| Method        | `REDIS_URL` Format                 | Use Case                |
|---------------|------------------------------------|-------------------------|
| Unix Socket   | `redis+unix:///path/to/redis.sock` | NixOS, local deployment |
| TCP           | `redis://host:6379`                | Docker, remote Redis    |
| TCP with auth | `redis://:password@host:6379`      | Authenticated Redis     |

---

## Reverse Proxy Configuration

Terrier requires a reverse proxy for SSL termination. The application binds to `127.0.0.1:3000` by default. You can customize this using:

1. The `services.nginx.virtualHosts` configuration in your `configuration.nix`, if using NixOS.

2. The `nginx.conf` file + the `TERRIER_PORT` environment variable in `.env` otherwise.

You can use the example [`nginx.conf`](examples/nginx.conf) as a starting point.
