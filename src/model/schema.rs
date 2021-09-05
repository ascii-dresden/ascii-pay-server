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
        receives_monthly_report -> Bool,
        allow_nfc_registration -> Bool,
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
        barcode -> Nullable<Varchar>,
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
    transaction (id) {
        id -> Uuid,
        account_id -> Uuid,
        cashier_id -> Nullable<Uuid>,
        total -> Int4,
        before_credit -> Int4,
        after_credit -> Int4,
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
    apple_wallet_pass,
    apple_wallet_registration,
    authentication_nfc,
    authentication_password,
    authentication_password_invitation,
    category,
    category_price,
    product,
    product_price,
    transaction,
    transaction_product,
);
