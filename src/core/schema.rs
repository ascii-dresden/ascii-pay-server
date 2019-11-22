table! {
    account (id) {
        id -> Text,
        credit -> Integer,
        limit -> Integer,
        name -> Nullable<Text>,
        mail -> Nullable<Text>,
        permission -> SmallInt,
    }
}

table! {
    authentication_barcode (account, code) {
        account -> Text,
        code -> Text,
    }
}

table! {
    authentication_password (account, username) {
        account -> Text,
        username -> Text,
        password -> Text,
    }
}

table! {
    category (id) {
        id -> Text,
        name -> Text,
    }
}

table! {
    category_price (category, validity_start) {
        category -> Text,
        validity_start -> Timestamp,
        value -> Integer,
    }
}

table! {
    product (id) {
        id -> Text,
        name -> Text,
        category -> Nullable<Text>,
        image -> Nullable<Text>,
    }
}

table! {
    product_price (product, validity_start) {
        product -> Text,
        validity_start -> Timestamp,
        value -> Integer,
    }
}

table! {
    session (id) {
        id -> Text,
        account_id -> Text,
        valid_until -> Timestamp,
    }
}

table! {
    transaction (id) {
        id -> Text,
        account -> Text,
        cashier -> Nullable<Text>,
        total -> Integer,
        date -> Timestamp,
    }
}

table! {
    transaction_product (transaction, product) {
        transaction -> Text,
        product -> Text,
        amount -> Integer,
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
