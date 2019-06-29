-- Your SQL goes here

CREATE TABLE `accounts` (
  `id` VARCHAR(100) PRIMARY KEY NOT NULL,
  `display` VARCHAR(32) NOT NULL,
  `credit` INT DEFAULT 0 NOT NULL,
  `limit` INT DEFAULT 0 NOT NULL,
  `created` DATETIME NOT NULL,
  `updated` DATETIME NOT NULL
);

CREATE TABLE `transactions` (
  `id` VARCHAR(100) PRIMARY KEY NOT NULL,
  `account_id` VARCHAR(100) UNIQUE NOT NULL,
  `amount` INT NOT NULL,
  `created` DATETIME NOT NULL
);
CREATE INDEX `transactions_account_index` ON `transactions` (`account_id`);

CREATE TABLE `users` (
  `id` VARCHAR(100) PRIMARY KEY NOT NULL,
  `account_id` VARCHAR(100) NOT NULL,
  `first_name` VARCHAR NOT NULL,
  `last_name` VARCHAR NOT NULL,
  `mail` VARCHAR UNIQUE NOT NULL,
  `password` VARCHAR NOT NULL,
  `created` DATETIME NOT NULL,
  `updated` DATETIME NOT NULL
);
CREATE INDEX `users_account_index` ON `users` (`account_id`);

CREATE TABLE `authentication_barcodes` (
  `id` VARCHAR(100) PRIMARY KEY NOT NULL,
  `account_id` VARCHAR(100) NOT NULL,
  `code` VARCHAR UNIQUE NOT NULL,
  `created` DATETIME NOT NULL
);
CREATE INDEX `authentication_barcodes_account_index` ON `authentication_barcodes` (`account_id`);
