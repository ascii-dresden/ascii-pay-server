CREATE TABLE "category" (
  "id" UUID PRIMARY KEY NOT NULL,
  "name" VARCHAR NOT NULL
);

CREATE TABLE "category_price" (
  "category_id" UUID NOT NULL,
  "validity_start" TIMESTAMP NOT NULL,
  "value" INT NOT NULL,
  PRIMARY KEY ("category_id", "validity_start")
);

CREATE TABLE "product" (
  "id" UUID PRIMARY KEY NOT NULL,
  "name" VARCHAR NOT NULL,
  "category" UUID,
  "image" VARCHAR,
  "barcode" VARCHAR
);

CREATE TABLE "product_price" (
  "product_id" UUID NOT NULL,
  "validity_start" TIMESTAMP NOT NULL,
  "value" INT NOT NULL,
  PRIMARY KEY ("product_id", "validity_start")
);
