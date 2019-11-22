table! {
    account (id) {
        id -> Varchar,
        credit -> Int4,
        minimum_credit -> Int4,
        name -> Nullable<Varchar>,
        mail -> Nullable<Varchar>,
        permission -> Int2,
    }
}

table! {
    authentication_barcode (account, code) {
        account -> Varchar,
        code -> Varchar,
    }
}

table! {
    authentication_password (account, username) {
        account -> Varchar,
        username -> Varchar,
        password -> Varchar,
    }
}

table! {
    category (id) {
        id -> Varchar,
        name -> Varchar,
    }
}

table! {
    category_price (category, validity_start) {
        category -> Varchar,
        validity_start -> Timestamp,
        value -> Int4,
    }
}

table! {
    product (id) {
        id -> Varchar,
        name -> Varchar,
        category -> Nullable<Varchar>,
        image -> Nullable<Varchar>,
    }
}

table! {
    product_price (product, validity_start) {
        product -> Varchar,
        validity_start -> Timestamp,
        value -> Int4,
    }
}

table! {
    session (id) {
        id -> Varchar,
        account_id -> Varchar,
        valid_until -> Timestamp,
    }
}

table! {
    transaction (id) {
        id -> Varchar,
        account -> Varchar,
        cashier -> Nullable<Varchar>,
        total -> Int4,
        date -> Timestamp,
    }
}

table! {
    transaction_product (transaction, product) {
        transaction -> Varchar,
        product -> Varchar,
        amount -> Int4,
    }
}

allow_tables_to_appear_in_same_query!(
    account,
    authentication_barcode,
    authentication_password,
    category,
    category_price,
    product,
    product_price,
    session,
    transaction,
    transaction_product,
);
