ALTER TABLE "authentication_nfc" DROP CONSTRAINT "authentication_nfc_pkey";
ALTER TABLE "authentication_nfc" ADD PRIMARY KEY ("account_id", "card_id");
