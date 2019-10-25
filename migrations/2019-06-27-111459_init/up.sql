CREATE TABLE `account` (
  `id` VARCHAR(100) PRIMARY KEY NOT NULL,
  `credit` INT DEFAULT 0 NOT NULL,
  `limit` INT DEFAULT 0 NOT NULL,
  `name` VARCHAR(64),
  `mail` VARCHAR(64)
);

CREATE TABLE `authentication_barcode` (
  `account` VARCHAR(100) NOT NULL,
  `code` VARCHAR UNIQUE NOT NULL,
  PRIMARY KEY (`account`, `code`)
);

CREATE TABLE `authentication_password` (
  `account` VARCHAR(100) NOT NULL,
  `username` VARCHAR UNIQUE NOT NULL,
  `password` VARCHAR NOT NULL,
  PRIMARY KEY (`account`, `username`)
);

CREATE TABLE `transaction` (
  `id` VARCHAR(100) PRIMARY KEY NOT NULL,
  `account` VARCHAR(100) NOT NULL,
  `total` INT NOT NULL,
  `date` DATETIME NOT NULL
);

CREATE TABLE `product` (
  `id` VARCHAR(100) PRIMARY KEY NOT NULL,
  `name` VARCHAR(64) NOT NULL
);

CREATE TABLE `price` (
  `product` VARCHAR(100) NOT NULL,
  `validity_start` DATETIME NOT NULL,
  `value` INT NOT NULL,
  PRIMARY KEY (`product`, `validity_start`)
);

CREATE TABLE `transaction_product` (
  `transaction` VARCHAR(100) NOT NULL,
  `product` VARCHAR(100) NOT NULL,
  `amount` INT NOT NULL,
  PRIMARY KEY (`transaction`, `product`)
);