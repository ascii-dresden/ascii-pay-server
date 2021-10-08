CREATE TABLE "category" (
  "id" UUID PRIMARY KEY NOT NULL,
  "name" VARCHAR NOT NULL,
  "price" INT NOT NULL,
  "pay_with_stamps" SMALLINT NOT NULL,
  "give_stamps" SMALLINT NOT NULL,
  "ordering" INT
);

CREATE TABLE "product" (
  "id" UUID PRIMARY KEY NOT NULL,
  "name" VARCHAR NOT NULL,
  "price" INT,
  "pay_with_stamps" SMALLINT,
  "give_stamps" SMALLINT,
  "category_id" UUID NOT NULL,
  "image" VARCHAR,
  "barcode" VARCHAR,
  "ordering" INT,
   CONSTRAINT fk_category
      FOREIGN KEY(category_id)
        REFERENCES category(id)
);
