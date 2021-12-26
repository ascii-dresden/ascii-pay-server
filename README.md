# ascii-pay

## Run for development

```bash
# Run only the database
docker-compose up -d postgres redis
cargo run
```

## Release build

```bash
# Starts database & service, performs initial migration if database doesn't exist yet
docker-compose up -d

# Stop service
docker-compose down
```
