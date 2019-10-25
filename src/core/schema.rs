table! {
    account (id) {
        id -> Text,
        credit -> Integer,
        limit -> Integer,
        name -> Nullable<Text>,
        mail -> Nullable<Text>,
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
    price (product, validity_start) {
        product -> Text,
        validity_start -> Timestamp,
        value -> Integer,
    }
}

table! {
    product (id) {
        id -> Text,
        name -> Text,
    }
}

table! {
    transaction (id) {
        id -> Text,
        account -> Text,
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
    price,
    product,
    transaction,
    transaction_product,
);
