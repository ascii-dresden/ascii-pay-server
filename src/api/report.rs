use aide::axum::routing::post_with;
use aide::axum::ApiRouter;
use aide::transform::TransformOperation;
use axum::extract::Path;
use chrono::{Duration, Utc};

use crate::database::AppState;
use crate::error::{ServiceError, ServiceResult};
use crate::models::Transaction;
use crate::request_state::RequestState;

pub fn router(app_state: AppState) -> ApiRouter {
    ApiRouter::new()
        .api_route(
            "/report/account/:id",
            post_with(report_account, report_account_docs),
        )
        .api_route(
            "/report/accounts",
            post_with(report_accounts, report_accounts_docs),
        )
        .with_state(app_state)
}

#[allow(unused_variables)]
async fn report_account(mut state: RequestState, Path(id): Path<u64>) -> ServiceResult<()> {
    state.session_require_admin_or_self(id)?;

    let end_date = Utc::now();
    let start_date = end_date - Duration::days(30);

    let account = state.db.get_account_by_id(id).await?;

    if let Some(account) = account {
        let transactions: Vec<Transaction> = state
            .db
            .get_transactions_by_account(id)
            .await?
            .into_iter()
            .filter(|t| start_date < t.timestamp && t.timestamp < end_date)
            .collect();

        #[cfg(feature = "mail")]
        if !account.email.is_empty() {
            if let Err(e) =
                crate::mail::send_monthly_report(&account, &transactions, start_date, end_date)
            {
                log::warn!("Could not send mail: {:?}", e);
            }
        }

        return Ok(());
    }
    Err(ServiceError::NotFound)
}

fn report_account_docs(op: TransformOperation) -> TransformOperation {
    op.description("Send mail report for account.")
        .tag("reports")
        .response::<200, ()>()
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["admin"])
}

#[allow(unused_variables)]
async fn report_accounts(mut state: RequestState) -> ServiceResult<()> {
    state.session_require_admin()?;

    let end_date = Utc::now();
    let start_date = end_date - Duration::days(30);

    let accounts = state.db.get_all_accounts().await?;

    for account in accounts {
        if !account.enable_monthly_mail_report {
            continue;
        }

        let transactions: Vec<Transaction> = state
            .db
            .get_transactions_by_account(account.id)
            .await?
            .into_iter()
            .filter(|t| start_date < t.timestamp && t.timestamp < end_date)
            .collect();

        #[cfg(feature = "mail")]
        if !account.email.is_empty() {
            if let Err(e) =
                crate::mail::send_monthly_report(&account, &transactions, start_date, end_date)
            {
                log::warn!("Could not send mail: {:?}", e);
            }
        }
    }

    Ok(())
}

fn report_accounts_docs(op: TransformOperation) -> TransformOperation {
    op.description("Send mail report for account.")
        .tag("reports")
        .response::<200, ()>()
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["admin"])
}
