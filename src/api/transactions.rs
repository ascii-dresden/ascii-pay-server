use aide::axum::routing::{get_with, post_with};
use aide::axum::ApiRouter;
use aide::transform::TransformOperation;
use axum::extract::Path;
use axum::Json;
use chrono::Utc;
use log::error;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::database::AppState;
use crate::error::{ServiceError, ServiceResult};
use crate::request_state::RequestState;
use crate::{models, wallet};

use super::account_auth_methods::AuthMethodTypeDto;
use super::accounts::{AccountDto, CoinAmountDto};
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
        .api_route(
            "/transactions",
            get_with(list_global_transactions, list_global_transactions_docs),
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
    pub authorized_by_account_id: Option<u64>,
    pub authorized_with_method: Option<AuthMethodTypeDto>,
    pub items: Vec<TransactionItemDto>,
}

impl From<&models::Transaction> for TransactionDto {
    fn from(value: &models::Transaction) -> Self {
        Self {
            id: value.id.to_owned(),
            timestamp: format!("{:?}", value.timestamp),
            account_id: value.account.to_owned(),
            authorized_by_account_id: value.authorized_by_account_id.to_owned(),
            authorized_with_method: value.authorized_with_method.map(|ref m| m.into()),
            items: value.items.iter().map(|i| i.into()).collect(),
        }
    }
}

pub async fn list_transactions(
    mut state: RequestState,
    Path(id): Path<u64>,
) -> ServiceResult<Json<Vec<TransactionDto>>> {
    state.session_require_admin_or_self(id)?;

    let transactions = state.db.get_transactions_by_account(id).await?;
    Ok(Json(transactions.iter().map(|t| t.into()).collect()))
}

fn list_transactions_docs(op: TransformOperation) -> TransformOperation {
    op.description("List all transactions for the given account.")
        .tag("transactions")
        .response::<200, Json<Vec<TransactionDto>>>()
        .response_with::<404, (), _>(|res| res.description("The requested account does not exist!"))
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["admin", "self"])
}

pub async fn list_global_transactions(
    mut state: RequestState,
) -> ServiceResult<Json<Vec<TransactionDto>>> {
    state.session_require_admin()?;

    let transactions = state.db.get_transactions().await?;
    Ok(Json(transactions.iter().map(|t| t.into()).collect()))
}

fn list_global_transactions_docs(op: TransformOperation) -> TransformOperation {
    op.description("List all transactions.")
        .tag("transactions")
        .response::<200, Json<Vec<TransactionDto>>>()
        .response_with::<404, (), _>(|res| res.description("The requested account does not exist!"))
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["admin", "self"])
}

pub async fn get_transaction(
    mut state: RequestState,
    Path((account_id, transaction_id)): Path<(u64, u64)>,
) -> ServiceResult<Json<TransactionDto>> {
    state.session_require_admin_or_self(account_id)?;

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
        .tag("transactions")
        .response::<200, Json<TransactionDto>>()
        .response_with::<404, (), _>(|res| {
            res.description("The requested account or transaction does not exist!")
        })
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["admin", "self"])
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

#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct PaymentResponseDto {
    pub account: AccountDto,
    pub transaction: TransactionDto,
}

async fn post_payment(
    mut state: RequestState,
    Path(id): Path<u64>,
    form: Json<PaymentDto>,
) -> ServiceResult<Json<PaymentResponseDto>> {
    state.session_require_admin_or_self(id)?;
    let session = state.session_require()?;

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
        authorization: Some(session),
    };

    let transaction = state.db.payment(payment, Utc::now(), true).await?;
    let account = state.db.get_account_by_id(id).await?;
    if let Some(account) = account {
        tokio::spawn(async move {
            if let Err(e) = wallet::send_update_notification(&mut state.db, id).await {
                error!("Could not send apns update! {:?}", e)
            }
        });

        return Ok(Json(PaymentResponseDto {
            account: AccountDto::from(&account),
            transaction: TransactionDto::from(&transaction),
        }));
    }

    Err(ServiceError::NotFound)
}

fn post_payment_docs(op: TransformOperation) -> TransformOperation {
    op.description("Execute a payment from the given account.")
        .tag("transactions")
        .response::<200, Json<PaymentResponseDto>>()
        .response_with::<404, (), _>(|res| res.description("The requested account does not exist!"))
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["admin", "self"])
}
