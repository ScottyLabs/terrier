# Terrier Backend

## Database

You should install `sea-orm-cli` using `cargo install sea-orm-cli`.

### Initializing

```bash
sea-orm-cli migrate init
```

### Generating a new migration

```bash
sea-orm-cli migrate generate --migration-dir ./migration <name>
```

### Applying migrations

```bash
sea-orm-cli migrate up --migration-dir ./migration
```

### Generating entities from database

```bash
sea-orm-cli generate entity -o ./src/entities --with-serde both --model-extra-derives "utoipa::ToSchema"
```
