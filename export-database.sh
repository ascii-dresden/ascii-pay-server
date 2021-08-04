#!/bin/bash

HOST="localhost"
PORT="54320"
USER="ascii"
PW="ascii"
DB="ascii-pay"

docker exec -i ascii-pay-postgres-dist /bin/bash -c "PGPASSWORD=$PW pg_dump --username $USER $DB"
