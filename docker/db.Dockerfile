FROM postgres

COPY ./migrations /migrations
WORKDIR /migrations
RUN mkdir -p /docker-entrypoint-initdb.d && for i in *; do cp "$i/up.sql" /docker-entrypoint-initdb.d/$i.sql; done

