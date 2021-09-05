CREATE TABLE "authentication_password" (
  "account_id" UUID PRIMARY KEY NOT NULL,
  "password" VARCHAR NOT NULL
);

CREATE TABLE "authentication_password_invitation" (
  "account_id" UUID PRIMARY KEY NOT NULL,
  "link" VARCHAR(100) UNIQUE NOT NULL,
  "valid_until" TIMESTAMP NOT NULL
);
