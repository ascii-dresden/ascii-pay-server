UPDATE "account" SET "mail"='' WHERE "mail" IS NULL;
ALTER TABLE "account" ALTER COLUMN "mail" SET NOT NULL;

UPDATE "account" SET "username"='' WHERE "username" IS NULL;
ALTER TABLE "account" ALTER COLUMN "username" SET NOT NULL;

UPDATE "account" SET "account_number"='' WHERE "account_number" IS NULL;
ALTER TABLE "account" ALTER COLUMN "account_number" SET NOT NULL;
