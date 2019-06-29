table! {
    accounts (id) {
        id -> Text,
        display -> Text,
        credit -> Integer,
        limit -> Integer,
        created -> Timestamp,
        updated -> Timestamp,
    }
}

table! {
    authentication_barcodes (id) {
        id -> Text,
        account_id -> Text,
        code -> Text,
        created -> Timestamp,
    }
}

table! {
    transactions (id) {
        id -> Text,
        account_id -> Text,
        amount -> Integer,
        created -> Timestamp,
    }
}

table! {
    users (id) {
        id -> Text,
        account_id -> Text,
        first_name -> Text,
        last_name -> Text,
        mail -> Text,
        password -> Text,
        created -> Timestamp,
        updated -> Timestamp,
    }
}

allow_tables_to_appear_in_same_query!(
    accounts,
    authentication_barcodes,
    transactions,
    users,
);
