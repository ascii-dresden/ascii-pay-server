-- This file should undo anything in `up.sql`

DROP TABLE `accounts`;

DROP INDEX `transactions_account_index`;
DROP TABLE `transactions`;

DROP INDEX `users_account_index`;
DROP TABLE `users`;

DROP INDEX `authentication_barcodes_account_index`;
DROP TABLE `authentication_barcodes`;
