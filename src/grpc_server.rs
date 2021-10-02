use std::ops::DerefMut;
use std::thread;

use log::info;
use uuid::Uuid;

use crate::grpc::authentication::*;
use crate::grpc::authentication_grpc::AsciiPayAuthentication;
use crate::grpc::authentication_grpc::AsciiPayAuthenticationServer;
use crate::identity_service::Identity;
use crate::repo::authentication_token as t;
use crate::utils::log_result;
use crate::utils::RedisPool;
use crate::utils::{DatabasePool, ServiceError};

struct AuthenticationImpl {
    database_pool: DatabasePool,
    redis_pool: RedisPool,
}

impl AuthenticationImpl {
    pub fn new(database_pool: DatabasePool, redis_pool: RedisPool) -> Self {
        Self {
            database_pool,
            redis_pool,
        }
    }
}

impl From<t::TokenType> for TokenType {
    fn from(item: t::TokenType) -> TokenType {
        match item {
            t::TokenType::AccountAccessToken => TokenType::ACCOUNT_ACCESS_TOKEN,
            t::TokenType::ProductId => TokenType::PRODUCT_ID,
        }
    }
}

impl From<t::NfcCardType> for NfcCardType {
    fn from(item: t::NfcCardType) -> NfcCardType {
        match item {
            t::NfcCardType::Generic => NfcCardType::GENERIC,
            t::NfcCardType::MifareDesfire => NfcCardType::MIFARE_DESFIRE,
        }
    }
}

impl AsciiPayAuthentication for AuthenticationImpl {
    fn authenticate_barcode(
        &self,
        _o: grpc::ServerHandlerContext,
        req: grpc::ServerRequestSingle<crate::grpc::authentication::AuthenticateBarcodeRequest>,
        resp: grpc::ServerResponseUnarySink<
            crate::grpc::authentication::AuthenticateBarcodeResponse,
        >,
    ) -> grpc::Result<()> {
        let database_conn = &self.database_pool.get().map_err(ServiceError::from)?;
        let mut redis_conn = self.redis_pool.get().map_err(ServiceError::from)?;
        let identity = Identity::from(req.metadata);

        let (token_type, token) = log_result(t::authenticate_barcode(
            database_conn,
            redis_conn.deref_mut(),
            &identity,
            req.message.get_code(),
        ))?;

        let mut response = AuthenticateBarcodeResponse::new();
        response.set_tokenType(token_type.into());
        response.set_token(token);

        resp.finish(response)
    }

    fn authenticate_nfc_type(
        &self,
        _o: grpc::ServerHandlerContext,
        req: grpc::ServerRequestSingle<crate::grpc::authentication::AuthenticateNfcTypeRequest>,
        resp: grpc::ServerResponseUnarySink<
            crate::grpc::authentication::AuthenticateNfcTypeResponse,
        >,
    ) -> grpc::Result<()> {
        let database_conn = &self.database_pool.get().map_err(ServiceError::from)?;
        let identity = Identity::from(req.metadata);

        let nfc_card_type = log_result(t::authenticate_nfc_type(
            database_conn,
            &identity,
            req.message.get_card_id(),
        ))?;

        let mut response = AuthenticateNfcTypeResponse::new();
        response.set_card_id(req.message.get_card_id().to_owned());
        response.set_tokenType(nfc_card_type.into());

        resp.finish(response)
    }

    fn authenticate_nfc_generic(
        &self,
        _o: grpc::ServerHandlerContext,
        req: grpc::ServerRequestSingle<crate::grpc::authentication::AuthenticateNfcGenericRequest>,
        resp: grpc::ServerResponseUnarySink<
            crate::grpc::authentication::AuthenticateNfcGenericResponse,
        >,
    ) -> grpc::Result<()> {
        let database_conn = &self.database_pool.get().map_err(ServiceError::from)?;
        let mut redis_conn = self.redis_pool.get().map_err(ServiceError::from)?;
        let identity = Identity::from(req.metadata);

        let (token_type, token) = log_result(t::authenticate_nfc_generic(
            database_conn,
            redis_conn.deref_mut(),
            &identity,
            req.message.get_card_id(),
        ))?;

        let mut response = AuthenticateNfcGenericResponse::new();
        response.set_card_id(req.message.get_card_id().to_owned());
        response.set_tokenType(token_type.into());
        response.set_token(token);

        resp.finish(response)
    }

    fn authenticate_nfc_mifare_desfire_phase1(
        &self,
        _o: grpc::ServerHandlerContext,
        req: grpc::ServerRequestSingle<
            crate::grpc::authentication::AuthenticateNfcMifareDesfirePhase1Request,
        >,
        resp: grpc::ServerResponseUnarySink<
            crate::grpc::authentication::AuthenticateNfcMifareDesfirePhase1Response,
        >,
    ) -> grpc::Result<()> {
        let database_conn = &self.database_pool.get().map_err(ServiceError::from)?;
        let mut redis_conn = self.redis_pool.get().map_err(ServiceError::from)?;
        let identity = Identity::from(req.metadata);

        let challenge = log_result(t::authenticate_nfc_mifare_desfire_phase1(
            database_conn,
            redis_conn.deref_mut(),
            &identity,
            req.message.get_card_id(),
            req.message.get_ek_rndB(),
        ))?;

        let mut response = AuthenticateNfcMifareDesfirePhase1Response::new();
        response.set_card_id(req.message.get_card_id().to_owned());
        response.set_dk_rndA_rndBshifted(challenge);

        resp.finish(response)
    }

    fn authenticate_nfc_mifare_desfire_phase2(
        &self,
        _o: grpc::ServerHandlerContext,
        req: grpc::ServerRequestSingle<
            crate::grpc::authentication::AuthenticateNfcMifareDesfirePhase2Request,
        >,
        resp: grpc::ServerResponseUnarySink<
            crate::grpc::authentication::AuthenticateNfcMifareDesfirePhase2Response,
        >,
    ) -> grpc::Result<()> {
        let database_conn = &self.database_pool.get().map_err(ServiceError::from)?;
        let mut redis_conn = self.redis_pool.get().map_err(ServiceError::from)?;
        let identity = Identity::from(req.metadata);

        let (session, (token_type, token)) =
            log_result(t::authenticate_nfc_mifare_desfire_phase2(
                database_conn,
                redis_conn.deref_mut(),
                &identity,
                req.message.get_card_id(),
                req.message.get_dk_rndA_rndBshifted(),
                req.message.get_ek_rndAshifted_card(),
            ))?;

        let mut response = AuthenticateNfcMifareDesfirePhase2Response::new();
        response.set_card_id(req.message.get_card_id().to_owned());
        response.set_session_key(session);
        response.set_tokenType(token_type.into());
        response.set_token(token);

        resp.finish(response)
    }

    fn authenticate_nfc_generic_init_card(
        &self,
        _o: grpc::ServerHandlerContext,
        req: grpc::ServerRequestSingle<
            crate::grpc::authentication::AuthenticateNfcGenericInitCardRequest,
        >,
        resp: grpc::ServerResponseUnarySink<
            crate::grpc::authentication::AuthenticateNfcGenericInitCardResponse,
        >,
    ) -> grpc::Result<()> {
        let database_conn = &self.database_pool.get().map_err(ServiceError::from)?;
        let identity = Identity::from(req.metadata);

        log_result(t::authenticate_nfc_generic_init_card(
            database_conn,
            &identity,
            req.message.get_card_id(),
            Uuid::parse_str(req.message.get_account_id()).map_err(ServiceError::from)?,
        ))?;

        let mut response = AuthenticateNfcGenericInitCardResponse::new();
        response.set_card_id(req.message.get_card_id().to_owned());

        resp.finish(response)
    }

    fn authenticate_nfc_mifare_desfire_init_card(
        &self,
        _o: grpc::ServerHandlerContext,
        req: grpc::ServerRequestSingle<
            crate::grpc::authentication::AuthenticateNfcMifareDesfireInitCardRequest,
        >,
        resp: grpc::ServerResponseUnarySink<
            crate::grpc::authentication::AuthenticateNfcMifareDesfireInitCardResponse,
        >,
    ) -> grpc::Result<()> {
        let database_conn = &self.database_pool.get().map_err(ServiceError::from)?;
        let identity = Identity::from(req.metadata);

        let key = log_result(t::authenticate_nfc_mifare_desfire_init_card(
            database_conn,
            &identity,
            req.message.get_card_id(),
            Uuid::parse_str(req.message.get_account_id()).map_err(ServiceError::from)?,
        ))?;

        let mut response = AuthenticateNfcMifareDesfireInitCardResponse::new();
        response.set_card_id(req.message.get_card_id().to_owned());
        response.set_key(key);

        resp.finish(response)
    }
}

pub fn start_tcp_server(database_pool: DatabasePool, redis_pool: RedisPool) {
    thread::spawn(move || {
        let port = 50051;

        let mut server_builder = grpc::ServerBuilder::new_plain();
        server_builder.http.set_port(port);
        server_builder.add_service(AsciiPayAuthenticationServer::new_service_def(
            AuthenticationImpl::new(database_pool, redis_pool),
        ));
        let server = server_builder.build().expect("build");

        info!("Start grpc server at {}", server.local_addr());

        loop {
            thread::park();
        }
    });
}
