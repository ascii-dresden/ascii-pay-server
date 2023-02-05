# ascii-pay-server

## Run integration tests

```bash
docker compose -f docker-compose.test.yml up -d --build
newman run ascii-pay-tests.postman_collection.json
docker compose -f docker-compose.test.yml down
```
