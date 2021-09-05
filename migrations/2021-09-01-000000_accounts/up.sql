CREATE TABLE "account" (
  "id" UUID PRIMARY KEY NOT NULL,
  "credit" INT DEFAULT 0 NOT NULL,
  "minimum_credit" INT DEFAULT 0 NOT NULL,
  "name" VARCHAR NOT NULL,
  "mail" VARCHAR,
  "username" VARCHAR,
  "account_number" VARCHAR,
  "permission" SMALLINT NOT NULL,
  "receives_monthly_report" BOOLEAN DEFAULT 'f' NOT NULL
);
