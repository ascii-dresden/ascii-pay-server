CREATE TABLE "authentication_nfc" (
  "account_id" UUID NOT NULL,
  "card_id" VARCHAR UNIQUE NOT NULL,
  "card_type" VARCHAR NOT NULL,
  "name" VARCHAR NOT NULL,
  "data" VARCHAR NOT NULL,
  PRIMARY KEY ("account_id", "card_id")
);
