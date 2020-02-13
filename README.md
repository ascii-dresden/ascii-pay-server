# ascii-pay

## Run

```bash
docker-compose up -d
cargo run
```

## Release build

```bash
# Build relase binary
cargo build --release

# Start db for inital setup
docker-compose --file docker-compose.release.yml up -d db

# Create db schema
diesel migration run

# Start dev server
cargo run

# Enter admin user and exit program

# Stop db
docker-compose --file docker-compose.release.yml down

# Start service
docker-compose --file docker-compose.release.yml up -d

# ascii pay server is not accessible via port 8080

# Stop service
docker-compose --file docker-compose.release.yml down
```
