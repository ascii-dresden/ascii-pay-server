ALTER TABLE "authentication_nfc" DROP CONSTRAINT "authentication_nfc_pkey";
ALTER TABLE "authentication_nfc" ADD PRIMARY KEY ("card_id");
