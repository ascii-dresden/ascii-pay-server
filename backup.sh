#!/bin/bash

HOST="localhost"
PORT="54320"
USER="ascii"
PW="ascii"
DB="ascii-pay"

PGPASSWORD=$PW pg_dump -h $HOST -p $PORT -U $USER -d $DB > db.sql 

DATE=`date +"%Y-%m-%d_%H-%M-%S"`

tar czf backup/backup_$DATE.tar.gz db.sql -C ./dist/ img

rm db.sql
