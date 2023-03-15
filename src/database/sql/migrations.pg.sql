-- see src/database/migration.rs for documentation on how migrations are handled
--##1 initial schema
CREATE TYPE tp_account_role AS ENUM ('basic', 'member', 'admin');
CREATE TABLE account (
    id BIGINT
        -- the id zero is reserved (for "unset")
        GENERATED ALWAYS AS IDENTITY (START WITH 1)
        PRIMARY KEY
        CHECK (id > 0),
    balance_cents INT NOT NULL,
    balance_coffee_stamps INT NOT NULL,
    balance_bottle_stamps INT NOT NULL,
    name TEXT NOT NULL,
    email TEXT NOT NULL,
    role tp_account_role NOT NULL
);

CREATE TYPE tp_auth_method_kind AS ENUM ('password', 'nfc', 'public_tab');
CREATE TABLE account_auth_method (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    account_id BIGINT NOT NULL,
    kind tp_auth_method_kind NOT NULL,
    -- the login key is an extra column to support indexed lookups by this key
    -- for example, this will be the username for user/password authentication
    -- or the card id for nfc auth
    login_key BYTEA NOT NULL,
    data JSONB NOT NULL,

    CONSTRAINT fk_account_id
        FOREIGN KEY(account_id)
            REFERENCES account(id)
            ON DELETE CASCADE
);
CREATE INDEX idx_account_auth_method_login_key ON account_auth_method(login_key);

CREATE TABLE product (
    id BIGINT
        -- the id zero is reserved (for "unset")
        GENERATED ALWAYS AS IDENTITY (START WITH 1)
        PRIMARY KEY
        CHECK (id > 0),
    name TEXT NOT NULL,
    price_cents INT,
    price_coffee_stamps INT,
    price_bottle_stamps INT,
    bonus_cents INT,
    bonus_coffee_stamps INT,
    bonus_bottle_stamps INT,
    nickname TEXT,
    image BYTEA,
    barcode TEXT,
    category TEXT,
    tags TEXT[]
);

CREATE TABLE transaction_item (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    effective_price_cents INT NOT NULL,
    effective_price_coffee_stamps INT NOT NULL,
    effective_price_bottle_stamps INT NOT NULL,
    product_id BIGINT,

    CONSTRAINT fk_product_id
        FOREIGN KEY(product_id)
            REFERENCES product(id)
            ON DELETE SET NULL
);

CREATE TABLE transaction (
    id BIGINT
      -- the id zero is reserved (for "unset")
        GENERATED ALWAYS AS IDENTITY (START WITH 1)
        PRIMARY KEY
        CHECK (id > 0),
    timestamp TIMESTAMP WITH TIME ZONE,
    account_id BIGINT,

    CONSTRAINT fk_account_id
        FOREIGN KEY(account_id)
            REFERENCES account(id)
            ON DELETE SET NULL
);
--##2 Unique constraint on login key
ALTER INDEX idx_account_auth_method_login_key RENAME TO idx_account_auth_method_login_key_old;
CREATE UNIQUE INDEX idx_account_auth_method_login_key ON account_auth_method(login_key);
ALTER TABLE account_auth_method ADD CONSTRAINT unique_account_auth_login_key UNIQUE USING INDEX idx_account_auth_method_login_key;
DROP INDEX idx_account_auth_method_login_key_old;

--##3 Remove kind column from auth method table
ALTER TABLE account_auth_method DROP COLUMN kind;

--##4 Add session storage
CREATE TABLE session (
    uuid UUID NOT NULL PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id BIGINT NOT NULL,
    auth_method tp_auth_method_kind NOT NULL,
    valid_until TIMESTAMPTZ NOT NULL,
    is_single_use BOOLEAN NOT NULL,

    CONSTRAINT fk_account_id
        FOREIGN KEY(account_id)
            REFERENCES account(id)
            ON DELETE CASCADE
);

--##5 Add password reset link
ALTER TYPE tp_auth_method_kind ADD VALUE 'password_reset_token';

--##6 Add image mimetype
ALTER TABLE product ADD COLUMN image_mimetype TEXT;

--##7 Product category must be not null
ALTER TABLE product ALTER COLUMN category SET NOT NULL;

--##8 Prices must be not null
ALTER TABLE product ALTER COLUMN price_cents SET NOT NULL;
ALTER TABLE product ALTER COLUMN price_coffee_stamps SET NOT NULL;
ALTER TABLE product ALTER COLUMN price_bottle_stamps SET NOT NULL;
ALTER TABLE product ALTER COLUMN bonus_cents SET NOT NULL;
ALTER TABLE product ALTER COLUMN bonus_coffee_stamps SET NOT NULL;
ALTER TABLE product ALTER COLUMN bonus_bottle_stamps SET NOT NULL;

--##9 Merge transaction table into transaction_item
ALTER TABLE transaction_item ADD COLUMN transaction_id BIGINT NOT NULL;
ALTER TABLE transaction_item ADD COLUMN timestamp TIMESTAMP WITH TIME ZONE NOT NULL;
ALTER TABLE transaction_item ADD COLUMN account_id BIGINT NOT NULL;

--##10 Add account id foreign key to transactions
ALTER TABLE transaction_item
    ADD CONSTRAINT fk_account_id
    FOREIGN KEY(account_id)
        REFERENCES account(id)
        ON DELETE SET NULL;

--##11 Delete transaction table
DROP TABLE transaction;

--##12 Create sequence for transaction ids
CREATE SEQUENCE transaction_id_seq AS BIGINT START WITH 1 NO CYCLE;

--##13 Allow deleting accounts with transactions
ALTER TABLE transaction_item ALTER COLUMN account_id DROP NOT NULL;

--##14 Add account settings
ALTER TABLE account ADD COLUMN enable_monthly_mail_report BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE account ADD COLUMN enable_automatic_stamp_usage BOOLEAN NOT NULL DEFAULT TRUE;
