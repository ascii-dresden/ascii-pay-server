use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::database::Database;
use crate::error::{ServiceError, ServiceResult};
use crate::models;

use super::accounts::CoinAmountDto;
use super::products::ProductDto;

pub fn router() -> Router<Database> {
    Router::new()
        .route("/account/:id/payment", post(post_payment))
        .route(
            "/account/:id/transaction/:transaction",
            get(get_transaction),
        )
        .route("/account/:id/transactions", get(list_transactions))
}

#[derive(Debug, PartialEq, Serialize)]
pub struct TransactionItemDto {
    pub effective_price: CoinAmountDto,
    pub product: Option<ProductDto>,
}

impl From<&models::TransactionItem> for TransactionItemDto {
    fn from(value: &models::TransactionItem) -> Self {
        let product = value.product.as_ref().map(|product| product.into());

        Self {
            effective_price: (&value.effective_price).into(),
            product,
        }
    }
}

#[derive(Debug, PartialEq, Serialize)]
pub struct TransactionDto {
    pub id: u64,
    pub timestamp: String,
    pub account_id: u64,
    pub items: Vec<TransactionItemDto>,
}

impl From<&models::Transaction> for TransactionDto {
    fn from(value: &models::Transaction) -> Self {
        Self {
            id: value.id.to_owned(),
            timestamp: format!("{:?}", value.timestamp),
            account_id: value.account.to_owned(),
            items: value.items.iter().map(|i| i.into()).collect(),
        }
    }
}

pub async fn list_transactions(
    State(database): State<Database>,
    Path(id): Path<u64>,
) -> ServiceResult<Json<Vec<TransactionDto>>> {
    let transactions = database.get_transactions_by_account(id).await?;
    Ok(Json(transactions.iter().map(|t| t.into()).collect()))
}

pub async fn get_transaction(
    State(database): State<Database>,
    Path((account_id, transaction_id)): Path<(u64, u64)>,
) -> ServiceResult<Json<TransactionDto>> {
    let transaction = database.get_transaction_by_id(transaction_id).await?;

    if let Some(transaction) = transaction {
        if transaction.account == account_id {
            return Ok(Json(TransactionDto::from(&transaction)));
        }
    }

    Err(ServiceError::NotFound)
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct PaymentItemDto {
    pub effective_price: CoinAmountDto,
    pub product_id: Option<u64>,
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct PaymentDto {
    pub items: Vec<PaymentItemDto>,
}

async fn post_payment(
    State(database): State<Database>,
    Path(id): Path<u64>,
    form: Json<PaymentDto>,
) -> ServiceResult<Json<TransactionDto>> {
    let form = form.0;

    let payment = models::Payment {
        account: id,
        items: form
            .items
            .into_iter()
            .map(|item| models::PaymentItem {
                effective_price: item.effective_price.into(),
                product_id: item.product_id,
            })
            .collect(),
    };

    let transaction = database.payment(payment).await?;
    Ok(Json(TransactionDto::from(&transaction)))
}
