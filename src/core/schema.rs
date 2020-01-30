table! {
    account (id) {
        id -> Uuid,
        credit -> Int4,
        minimum_credit -> Int4,
        name -> Varchar,
        mail -> Nullable<Varchar>,
        username -> Nullable<Varchar>,
        account_number -> Nullable<Varchar>,
        permission -> Int2,
    }
}

table! {
    authentication_barcode (account_id) {
        account_id -> Uuid,
        code -> Varchar,
    }
}

table! {
    authentication_nfc (account_id) {
        account_id -> Uuid,
        card_id -> Varchar,
        key -> Nullable<Varchar>,
        secret -> Nullable<Varchar>,
    }
}

table! {
    authentication_nfc_write_key (account_id, card_id) {
        account_id -> Uuid,
        card_id -> Varchar,
    }
}

table! {
    authentication_password (account_id) {
        account_id -> Uuid,
        password -> Varchar,
    }
}

table! {
    authentication_password_invitation (account_id) {
        account_id -> Uuid,
        link -> Varchar,
        valid_until -> Timestamp,
    }
}

table! {
    category (id) {
        id -> Uuid,
        name -> Varchar,
    }
}

table! {
    category_price (category_id, validity_start) {
        category_id -> Uuid,
        validity_start -> Timestamp,
        value -> Int4,
    }
}

table! {
    product (id) {
        id -> Uuid,
        name -> Varchar,
        category -> Nullable<Uuid>,
        image -> Nullable<Varchar>,
    }
}

table! {
    product_barcode (product_id) {
        product_id -> Uuid,
        code -> Varchar,
    }
}

table! {
    product_price (product_id, validity_start) {
        product_id -> Uuid,
        validity_start -> Timestamp,
        value -> Int4,
    }
}

table! {
    session (id) {
        id -> Varchar,
        account_id -> Uuid,
        valid_until -> Timestamp,
    }
}

table! {
    transaction (id) {
        id -> Uuid,
        account_id -> Uuid,
        cashier_id -> Nullable<Uuid>,
        total -> Int4,
        date -> Timestamp,
    }
}

table! {
    transaction_product (transaction, product_id) {
        transaction -> Uuid,
        product_id -> Uuid,
        amount -> Int4,
    }
}

allow_tables_to_appear_in_same_query!(
    account,
    authentication_barcode,
    authentication_nfc,
    authentication_nfc_write_key,
    authentication_password,
    authentication_password_invitation,
    category,
    category_price,
    product,
    product_barcode,
    product_price,
    session,
    transaction,
    transaction_product,
);
