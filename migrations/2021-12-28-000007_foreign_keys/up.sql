ALTER TABLE "authentication_password"
ADD CONSTRAINT "account_authentication_password"
FOREIGN KEY ("account_id")
REFERENCES "account" ("id");

ALTER TABLE "authentication_nfc"
ADD CONSTRAINT "account_authentication_nfc"
FOREIGN KEY ("account_id")
REFERENCES "account" ("id");

ALTER TABLE "transaction"
ADD CONSTRAINT "account_transaction"
FOREIGN KEY ("account_id")
REFERENCES "account" ("id");
