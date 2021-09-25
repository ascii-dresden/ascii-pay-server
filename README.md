# ascii-pay

## Run for development

```bash
# Run only the database
docker-compose up -d postgres
cargo run
```

## Release build

```bash
# Starts database & service, performs initial migration if database doesn't exist yet
docker-compose up -d

# ascii pay server is now accessible via port 8080
# Add admin user and reload page

# Stop service
docker-compose down
```
