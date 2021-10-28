ALTER TABLE "account" ALTER COLUMN "mail" DROP NOT NULL;
UPDATE "account" SET "mail" = NULL WHERE "mail" = '';

ALTER TABLE "account" ALTER COLUMN "username" DROP NOT NULL;
UPDATE "account" SET "username" = NULL WHERE "username" = '';

ALTER TABLE "account" ALTER COLUMN "account_number" DROP NOT NULL;
UPDATE "account" SET "account_number" = NULL WHERE "account_number" = '';
