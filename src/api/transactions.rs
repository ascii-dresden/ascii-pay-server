use aide::axum::routing::{get_with, post_with};
use aide::axum::ApiRouter;
use aide::transform::TransformOperation;
use axum::extract::Path;
use axum::Json;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::database::AppState;
use crate::error::{ServiceError, ServiceResult};
use crate::{models, RequestState};

use super::accounts::CoinAmountDto;
use super::products::ProductDto;

pub fn router(app_state: AppState) -> ApiRouter {
    ApiRouter::new()
        .api_route(
            "/account/:id/payment",
            post_with(post_payment, post_payment_docs),
        )
        .api_route(
            "/account/:id/transaction/:transaction",
            get_with(get_transaction, get_transaction_docs),
        )
        .api_route(
            "/account/:id/transactions",
            get_with(list_transactions, list_transactions_docs),
        )
        .with_state(app_state)
}

#[derive(Debug, PartialEq, Serialize, JsonSchema)]
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

#[derive(Debug, PartialEq, Serialize, JsonSchema)]
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
    state: RequestState,
    Path(id): Path<u64>,
) -> ServiceResult<Json<Vec<TransactionDto>>> {
    let transactions = state.db.get_transactions_by_account(id).await?;
    Ok(Json(transactions.iter().map(|t| t.into()).collect()))
}

fn list_transactions_docs(op: TransformOperation) -> TransformOperation {
    op.description("List all transactions for the given account.")
        .response::<200, Json<Vec<TransactionDto>>>()
        .response::<404, ()>()
        .response::<500, ()>()
}

pub async fn get_transaction(
    state: RequestState,
    Path((account_id, transaction_id)): Path<(u64, u64)>,
) -> ServiceResult<Json<TransactionDto>> {
    let transaction = state.db.get_transaction_by_id(transaction_id).await?;

    if let Some(transaction) = transaction {
        if transaction.account == account_id {
            return Ok(Json(TransactionDto::from(&transaction)));
        }
    }

    Err(ServiceError::NotFound)
}

fn get_transaction_docs(op: TransformOperation) -> TransformOperation {
    op.description("Get a transactions from the given account.")
        .response::<200, Json<TransactionDto>>()
        .response::<404, ()>()
        .response::<500, ()>()
}

#[derive(Debug, PartialEq, Deserialize, JsonSchema)]
pub struct PaymentItemDto {
    pub effective_price: CoinAmountDto,
    pub product_id: Option<u64>,
}

#[derive(Debug, PartialEq, Deserialize, JsonSchema)]
pub struct PaymentDto {
    pub items: Vec<PaymentItemDto>,
}

async fn post_payment(
    state: RequestState,
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

    let transaction = state.db.payment(payment).await?;
    Ok(Json(TransactionDto::from(&transaction)))
}

fn post_payment_docs(op: TransformOperation) -> TransformOperation {
    op.description("Execute a payment from the given account.")
        .response::<200, Json<TransactionDto>>()
        .response::<404, ()>()
        .response::<500, ()>()
}
