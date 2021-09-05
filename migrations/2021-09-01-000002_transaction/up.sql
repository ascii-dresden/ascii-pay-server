CREATE TABLE "transaction" (
  "id" UUID PRIMARY KEY NOT NULL,
  "account_id" UUID NOT NULL,
  "cashier_id" UUID,
  "total" INT NOT NULL,
  "before_credit" INT NOT NULL,
  "after_credit" INT NOT NULL,
  "date" TIMESTAMP NOT NULL
);

CREATE TABLE "transaction_product" (
  "transaction" UUID NOT NULL,
  "product_id" UUID NOT NULL,
  "amount" INT NOT NULL,
  PRIMARY KEY ("transaction", "product_id")
);
