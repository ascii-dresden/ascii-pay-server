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
        use_digital_stamps -> Bool,
        coffee_stamps -> Int4,
        bottle_stamps -> Int4,
        receives_monthly_report -> Bool,
    }
}

table! {
    apple_wallet_pass (serial_number) {
        serial_number -> Uuid,
        authentication_token -> Uuid,
        qr_code -> Varchar,
        pass_type_id -> Varchar,
        updated_at -> Int4,
    }
}

table! {
    apple_wallet_registration (device_id, serial_number) {
        device_id -> Varchar,
        serial_number -> Uuid,
        push_token -> Varchar,
        pass_type_id -> Varchar,
    }
}

table! {
    authentication_nfc (account_id, card_id) {
        account_id -> Uuid,
        card_id -> Varchar,
        card_type -> Varchar,
        name -> Varchar,
        data -> Varchar,
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
        price -> Int4,
        pay_with_stamps -> Int2,
        give_stamps -> Int2,
        ordering -> Nullable<Int4>,
    }
}

table! {
    product (id) {
        id -> Uuid,
        name -> Varchar,
        price -> Nullable<Int4>,
        pay_with_stamps -> Nullable<Int2>,
        give_stamps -> Nullable<Int2>,
        category_id -> Uuid,
        image -> Nullable<Varchar>,
        barcode -> Nullable<Varchar>,
        ordering -> Nullable<Int4>,
    }
}

table! {
    transaction (id) {
        id -> Uuid,
        account_id -> Uuid,
        total -> Int4,
        before_credit -> Int4,
        after_credit -> Int4,
        coffee_stamps -> Int4,
        before_coffee_stamps -> Int4,
        after_coffee_stamps -> Int4,
        bottle_stamps -> Int4,
        before_bottle_stamps -> Int4,
        after_bottle_stamps -> Int4,
        date -> Timestamp,
    }
}

table! {
    transaction_item (transaction_id, index) {
        transaction_id -> Uuid,
        index -> Int4,
        price -> Int4,
        pay_with_stamps -> Int2,
        give_stamps -> Int2,
        product_id -> Nullable<Uuid>,
    }
}

joinable!(product -> category (category_id));
joinable!(transaction_item -> transaction (transaction_id));

allow_tables_to_appear_in_same_query!(
    account,
    apple_wallet_pass,
    apple_wallet_registration,
    authentication_nfc,
    authentication_password,
    authentication_password_invitation,
    category,
    product,
    transaction,
    transaction_item,
);
