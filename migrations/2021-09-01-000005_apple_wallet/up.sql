CREATE TABLE "apple_wallet_pass" (
  "serial_number" UUID PRIMARY KEY NOT NULL,
  "authentication_token" UUID NOT NULL,
  "qr_code" VARCHAR NOT NULL,
  "pass_type_id" VARCHAR NOT NULL,
  "updated_at" INT NOT NULL
);

CREATE TABLE "apple_wallet_registration" (
  "device_id" VARCHAR NOT NULL,
  "serial_number" UUID NOT NULL,
  "push_token" VARCHAR NOT NULL,
  "pass_type_id" VARCHAR NOT NULL,
  PRIMARY KEY ("device_id", "serial_number")
);
