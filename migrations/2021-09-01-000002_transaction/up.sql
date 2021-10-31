CREATE TABLE "transaction" (
  "id" UUID PRIMARY KEY NOT NULL,
  "account_id" UUID NOT NULL,
  "total" INT NOT NULL,
  "before_credit" INT NOT NULL,
  "after_credit" INT NOT NULL,
  "coffee_stamps" INT NOT NULL,
  "before_coffee_stamps" INT NOT NULL,
  "after_coffee_stamps" INT NOT NULL,
  "bottle_stamps" INT NOT NULL,
  "before_bottle_stamps" INT NOT NULL,
  "after_bottle_stamps" INT NOT NULL,
  "date" TIMESTAMP NOT NULL
);

CREATE TABLE "transaction_item" (
  "transaction_id" UUID NOT NULL,
  "index" INT NOT NULL,
  "price" INT NOT NULL,
  "pay_with_stamps" SMALLINT NOT NULL,
  "give_stamps" SMALLINT NOT NULL,
  "product_id" VARCHAR NOT NULL,
  PRIMARY KEY ("transaction_id", "index"),
   CONSTRAINT fk_transaction
      FOREIGN KEY(transaction_id)
        REFERENCES transaction(id)
);
