CREATE TABLE "account" (
  "id" UUID PRIMARY KEY NOT NULL,
  "credit" INT DEFAULT 0 NOT NULL,
  "minimum_credit" INT DEFAULT 0 NOT NULL,
  "name" VARCHAR(64) NOT NULL,
  "mail" VARCHAR(64),
  "username" VARCHAR(64),
  "account_number" VARCHAR(64),
  "permission" SMALLINT NOT NULL
);

CREATE TABLE "authentication_barcode" (
  "account_id" UUID PRIMARY KEY NOT NULL,
  "code" VARCHAR UNIQUE NOT NULL
);

CREATE TABLE "authentication_password" (
  "account_id" UUID PRIMARY KEY NOT NULL,
  "password" VARCHAR NOT NULL
);

CREATE TABLE "authentication_password_invitation" (
  "account_id" UUID PRIMARY KEY NOT NULL,
  "link" VARCHAR(100) UNIQUE NOT NULL,
  "valid_until" TIMESTAMP NOT NULL
);

CREATE TABLE "authentication_nfc" (
  "account_id" UUID PRIMARY KEY NOT NULL,
  "card_id" VARCHAR UNIQUE NOT NULL,
  "key" VARCHAR,
  "secret" VARCHAR
);

CREATE TABLE "authentication_nfc_write_key" (
  "account_id" UUID NOT NULL,
  "card_id" VARCHAR NOT NULL,
  PRIMARY KEY ("account_id", "card_id")
);

CREATE TABLE "transaction" (
  "id" UUID PRIMARY KEY NOT NULL,
  "account_id" UUID NOT NULL,
  "cashier_id" UUID,
  "total" INT NOT NULL,
  "before_credit" INT NOT NULL,
  "after_credit" INT NOT NULL,
  "date" TIMESTAMP NOT NULL
);

CREATE TABLE "category" (
  "id" UUID PRIMARY KEY NOT NULL,
  "name" VARCHAR(64) NOT NULL
);

CREATE TABLE "category_price" (
  "category_id" UUID NOT NULL,
  "validity_start" TIMESTAMP NOT NULL,
  "value" INT NOT NULL,
  PRIMARY KEY ("category_id", "validity_start")
);

CREATE TABLE "product" (
  "id" UUID PRIMARY KEY NOT NULL,
  "name" VARCHAR(64) NOT NULL,
  "category" UUID,
  "image" VARCHAR(105)
);

CREATE TABLE "product_barcode" (
  "product_id" UUID PRIMARY KEY NOT NULL,
  "code" VARCHAR UNIQUE NOT NULL
);

CREATE TABLE "product_price" (
  "product_id" UUID NOT NULL,
  "validity_start" TIMESTAMP NOT NULL,
  "value" INT NOT NULL,
  PRIMARY KEY ("product_id", "validity_start")
);

CREATE TABLE "transaction_product" (
  "transaction" UUID NOT NULL,
  "product_id" UUID NOT NULL,
  "amount" INT NOT NULL,
  PRIMARY KEY ("transaction", "product_id")
);

CREATE TABLE "session" (
  "id" VARCHAR PRIMARY KEY NOT NULL,
  "account_id" UUID NOT NULL,
  "valid_until" TIMESTAMP NOT NULL
);
