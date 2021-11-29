use std::sync::Arc;
use std::thread;

use futures_util::TryFutureExt;
use grpcio::{ChannelBuilder, Environment, ResourceQuota, RpcStatus, ServerBuilder};
use log::{error, info};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::grpc::authentication::*;
use crate::grpc::authentication_grpc::{create_ascii_pay_authentication, AsciiPayAuthentication};
use crate::identity_service::Identity;
use crate::repo::authentication_token as t;
use crate::utils::{env, log_result, DatabasePool, RedisPool, ServiceError, ServiceResult};

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

#[derive(Debug)]
enum WorkRequest {
    Barcode {
        identity: Identity,
        request: crate::grpc::authentication::AuthenticateBarcodeRequest,
        response:
            mpsc::Sender<ServiceResult<crate::grpc::authentication::AuthenticateBarcodeResponse>>,
    },
    NfcType {
        identity: Identity,
        request: crate::grpc::authentication::AuthenticateNfcTypeRequest,
        response:
            mpsc::Sender<ServiceResult<crate::grpc::authentication::AuthenticateNfcTypeResponse>>,
    },
    NfcGeneric {
        identity: Identity,
        request: crate::grpc::authentication::AuthenticateNfcGenericRequest,
        response: mpsc::Sender<
            ServiceResult<crate::grpc::authentication::AuthenticateNfcGenericResponse>,
        >,
    },
    NfcMifareDesfirePhase1 {
        identity: Identity,
        request: crate::grpc::authentication::AuthenticateNfcMifareDesfirePhase1Request,
        response: mpsc::Sender<
            ServiceResult<crate::grpc::authentication::AuthenticateNfcMifareDesfirePhase1Response>,
        >,
    },
    NfcMifareDesfirePhase2 {
        identity: Identity,
        request: crate::grpc::authentication::AuthenticateNfcMifareDesfirePhase2Request,
        response: mpsc::Sender<
            ServiceResult<crate::grpc::authentication::AuthenticateNfcMifareDesfirePhase2Response>,
        >,
    },
    NfcGenericInitCard {
        identity: Identity,
        request: crate::grpc::authentication::AuthenticateNfcGenericInitCardRequest,
        response: mpsc::Sender<
            ServiceResult<crate::grpc::authentication::AuthenticateNfcGenericInitCardResponse>,
        >,
    },
    NfcMifareDesfireInitCard {
        identity: Identity,
        request: crate::grpc::authentication::AuthenticateNfcMifareDesfireInitCardRequest,
        response: mpsc::Sender<
            ServiceResult<
                crate::grpc::authentication::AuthenticateNfcMifareDesfireInitCardResponse,
            >,
        >,
    },
}

#[derive(Debug, Clone)]
struct AuthenticationRunner {
    database_pool: DatabasePool,
    redis_pool: RedisPool,
}

impl AuthenticationRunner {
    pub fn new(database_pool: DatabasePool, redis_pool: RedisPool) -> Self {
        Self {
            database_pool,
            redis_pool,
        }
    }

    fn run(self) -> mpsc::Sender<WorkRequest> {
        let (send, mut recv) = mpsc::channel(16);

        tokio::spawn(async move {
            while let Some(work_request) = recv.recv().await {
                if let Err(e) = self.run_work_request(work_request).await {
                    error!("Cannot execute grpc work request: {:?}", e);
                }
            }
        });

        send
    }

    async fn run_work_request(&self, work_request: WorkRequest) -> ServiceResult<()> {
        match work_request {
            WorkRequest::Barcode {
                identity,
                request,
                response,
            } => {
                let result = self.authenticate_barcode_async(identity, request).await;
                response.send(result).await?;
            }
            WorkRequest::NfcType {
                identity,
                request,
                response,
            } => {
                let result = self.authenticate_nfc_type_async(identity, request).await;
                response.send(result).await?;
            }
            WorkRequest::NfcGeneric {
                identity,
                request,
                response,
            } => {
                let result = self.authenticate_nfc_generic_async(identity, request).await;
                response.send(result).await?;
            }
            WorkRequest::NfcMifareDesfirePhase1 {
                identity,
                request,
                response,
            } => {
                let result = self
                    .authenticate_nfc_mifare_desfire_phase1_async(identity, request)
                    .await;
                response.send(result).await?;
            }
            WorkRequest::NfcMifareDesfirePhase2 {
                identity,
                request,
                response,
            } => {
                let result = self
                    .authenticate_nfc_mifare_desfire_phase2_async(identity, request)
                    .await;
                response.send(result).await?;
            }
            WorkRequest::NfcGenericInitCard {
                identity,
                request,
                response,
            } => {
                let result = self
                    .authenticate_nfc_generic_init_card_async(identity, request)
                    .await;
                response.send(result).await?;
            }
            WorkRequest::NfcMifareDesfireInitCard {
                identity,
                request,
                response,
            } => {
                let result = self
                    .authenticate_nfc_mifare_desfire_init_card_async(identity, request)
                    .await;
                response.send(result).await?;
            }
        }
        Ok(())
    }

    async fn authenticate_barcode_async(
        &self,
        identity: Identity,
        req: crate::grpc::authentication::AuthenticateBarcodeRequest,
    ) -> ServiceResult<crate::grpc::authentication::AuthenticateBarcodeResponse> {
        let (token_type, token) = log_result(
            t::authenticate_barcode(
                &self.database_pool,
                &self.redis_pool,
                &identity,
                req.get_code(),
            )
            .await,
        )?;

        let mut response = AuthenticateBarcodeResponse::new();
        response.set_tokenType(token_type.into());
        response.set_token(token);

        Ok(response)
    }

    async fn authenticate_nfc_type_async(
        &self,
        identity: Identity,
        req: crate::grpc::authentication::AuthenticateNfcTypeRequest,
    ) -> ServiceResult<crate::grpc::authentication::AuthenticateNfcTypeResponse> {
        let nfc_card_type = log_result(
            t::authenticate_nfc_type(&self.database_pool, &identity, req.get_card_id()).await,
        )?;

        println!("{:?}", req.get_card_id());
        println!("{:?}", nfc_card_type);

        let mut response = AuthenticateNfcTypeResponse::new();
        response.set_card_id(req.get_card_id().to_owned());
        response.set_tokenType(nfc_card_type.into());

        Ok(response)
    }

    async fn authenticate_nfc_generic_async(
        &self,
        identity: Identity,
        req: crate::grpc::authentication::AuthenticateNfcGenericRequest,
    ) -> ServiceResult<crate::grpc::authentication::AuthenticateNfcGenericResponse> {
        let (token_type, token) = log_result(
            t::authenticate_nfc_generic(
                &self.database_pool,
                &self.redis_pool,
                &identity,
                req.get_card_id(),
            )
            .await,
        )?;

        let mut response = AuthenticateNfcGenericResponse::new();
        response.set_card_id(req.get_card_id().to_owned());
        response.set_tokenType(token_type.into());
        response.set_token(token);

        Ok(response)
    }

    async fn authenticate_nfc_mifare_desfire_phase1_async(
        &self,
        identity: Identity,
        req: crate::grpc::authentication::AuthenticateNfcMifareDesfirePhase1Request,
    ) -> ServiceResult<crate::grpc::authentication::AuthenticateNfcMifareDesfirePhase1Response>
    {
        let challenge = log_result(
            t::authenticate_nfc_mifare_desfire_phase1(
                &self.database_pool,
                &self.redis_pool,
                &identity,
                req.get_card_id(),
                req.get_ek_rndB(),
            )
            .await,
        )?;

        let mut response = AuthenticateNfcMifareDesfirePhase1Response::new();
        response.set_card_id(req.get_card_id().to_owned());
        response.set_dk_rndA_rndBshifted(challenge);

        Ok(response)
    }

    async fn authenticate_nfc_mifare_desfire_phase2_async(
        &self,
        identity: Identity,
        req: crate::grpc::authentication::AuthenticateNfcMifareDesfirePhase2Request,
    ) -> ServiceResult<crate::grpc::authentication::AuthenticateNfcMifareDesfirePhase2Response>
    {
        let (session, (token_type, token)) = log_result(
            t::authenticate_nfc_mifare_desfire_phase2(
                &self.database_pool,
                &self.redis_pool,
                &identity,
                req.get_card_id(),
                req.get_dk_rndA_rndBshifted(),
                req.get_ek_rndAshifted_card(),
            )
            .await,
        )?;

        let mut response = AuthenticateNfcMifareDesfirePhase2Response::new();
        response.set_card_id(req.get_card_id().to_owned());
        response.set_session_key(session);
        response.set_tokenType(token_type.into());
        response.set_token(token);

        Ok(response)
    }

    async fn authenticate_nfc_generic_init_card_async(
        &self,
        identity: Identity,
        req: crate::grpc::authentication::AuthenticateNfcGenericInitCardRequest,
    ) -> ServiceResult<crate::grpc::authentication::AuthenticateNfcGenericInitCardResponse> {
        log_result(
            t::authenticate_nfc_generic_init_card(
                &self.database_pool,
                &identity,
                req.get_card_id(),
                Uuid::parse_str(req.get_account_id())?,
            )
            .await,
        )?;

        let mut response = AuthenticateNfcGenericInitCardResponse::new();
        response.set_card_id(req.get_card_id().to_owned());

        Ok(response)
    }

    async fn authenticate_nfc_mifare_desfire_init_card_async(
        &self,
        identity: Identity,
        req: crate::grpc::authentication::AuthenticateNfcMifareDesfireInitCardRequest,
    ) -> ServiceResult<crate::grpc::authentication::AuthenticateNfcMifareDesfireInitCardResponse>
    {
        let key = log_result(
            t::authenticate_nfc_mifare_desfire_init_card(
                &self.database_pool,
                &identity,
                req.get_card_id(),
                Uuid::parse_str(req.get_account_id())?,
            )
            .await,
        )?;

        let mut response = AuthenticateNfcMifareDesfireInitCardResponse::new();
        response.set_card_id(req.get_card_id().to_owned());
        response.set_key(key);

        Ok(response)
    }
}

#[derive(Debug, Clone)]
struct AuthenticationImpl {
    send: mpsc::Sender<WorkRequest>,
}

impl AuthenticationImpl {
    fn new(send: mpsc::Sender<WorkRequest>) -> Self {
        Self { send }
    }

    fn authenticate_barcode_blocking(
        &self,
        identity: Identity,
        req: crate::grpc::authentication::AuthenticateBarcodeRequest,
    ) -> ServiceResult<crate::grpc::authentication::AuthenticateBarcodeResponse> {
        let (send, mut recv) = mpsc::channel(1);
        self.send.blocking_send(WorkRequest::Barcode {
            identity,
            request: req,
            response: send,
        })?;
        match recv.blocking_recv() {
            Some(result) => result,
            None => Err(ServiceError::NoneError),
        }
    }

    fn authenticate_nfc_type_blocking(
        &self,
        identity: Identity,
        req: crate::grpc::authentication::AuthenticateNfcTypeRequest,
    ) -> ServiceResult<crate::grpc::authentication::AuthenticateNfcTypeResponse> {
        let (send, mut recv) = mpsc::channel(1);
        self.send.blocking_send(WorkRequest::NfcType {
            identity,
            request: req,
            response: send,
        })?;
        match recv.blocking_recv() {
            Some(result) => result,
            None => Err(ServiceError::NoneError),
        }
    }

    fn authenticate_nfc_generic_blocking(
        &self,
        identity: Identity,
        req: crate::grpc::authentication::AuthenticateNfcGenericRequest,
    ) -> ServiceResult<crate::grpc::authentication::AuthenticateNfcGenericResponse> {
        let (send, mut recv) = mpsc::channel(1);
        self.send.blocking_send(WorkRequest::NfcGeneric {
            identity,
            request: req,
            response: send,
        })?;
        match recv.blocking_recv() {
            Some(result) => result,
            None => Err(ServiceError::NoneError),
        }
    }

    fn authenticate_nfc_mifare_desfire_phase1_blocking(
        &self,
        identity: Identity,
        req: crate::grpc::authentication::AuthenticateNfcMifareDesfirePhase1Request,
    ) -> ServiceResult<crate::grpc::authentication::AuthenticateNfcMifareDesfirePhase1Response>
    {
        let (send, mut recv) = mpsc::channel(1);
        self.send
            .blocking_send(WorkRequest::NfcMifareDesfirePhase1 {
                identity,
                request: req,
                response: send,
            })?;
        match recv.blocking_recv() {
            Some(result) => result,
            None => Err(ServiceError::NoneError),
        }
    }

    fn authenticate_nfc_mifare_desfire_phase2_blocking(
        &self,
        identity: Identity,
        req: crate::grpc::authentication::AuthenticateNfcMifareDesfirePhase2Request,
    ) -> ServiceResult<crate::grpc::authentication::AuthenticateNfcMifareDesfirePhase2Response>
    {
        let (send, mut recv) = mpsc::channel(1);
        self.send
            .blocking_send(WorkRequest::NfcMifareDesfirePhase2 {
                identity,
                request: req,
                response: send,
            })?;
        match recv.blocking_recv() {
            Some(result) => result,
            None => Err(ServiceError::NoneError),
        }
    }

    fn authenticate_nfc_generic_init_card_blocking(
        &self,
        identity: Identity,
        req: crate::grpc::authentication::AuthenticateNfcGenericInitCardRequest,
    ) -> ServiceResult<crate::grpc::authentication::AuthenticateNfcGenericInitCardResponse> {
        let (send, mut recv) = mpsc::channel(1);
        self.send.blocking_send(WorkRequest::NfcGenericInitCard {
            identity,
            request: req,
            response: send,
        })?;
        match recv.blocking_recv() {
            Some(result) => result,
            None => Err(ServiceError::NoneError),
        }
    }

    fn authenticate_nfc_mifare_desfire_init_card_blocking(
        &self,
        identity: Identity,
        req: crate::grpc::authentication::AuthenticateNfcMifareDesfireInitCardRequest,
    ) -> ServiceResult<crate::grpc::authentication::AuthenticateNfcMifareDesfireInitCardResponse>
    {
        let (send, mut recv) = mpsc::channel(1);
        self.send
            .blocking_send(WorkRequest::NfcMifareDesfireInitCard {
                identity,
                request: req,
                response: send,
            })?;
        match recv.blocking_recv() {
            Some(result) => result,
            None => Err(ServiceError::NoneError),
        }
    }
}

impl AsciiPayAuthentication for AuthenticationImpl {
    fn authenticate_barcode(
        &mut self,
        ctx: grpcio::RpcContext,
        req: crate::grpc::authentication::AuthenticateBarcodeRequest,
        sink: grpcio::UnarySink<crate::grpc::authentication::AuthenticateBarcodeResponse>,
    ) {
        let identity = Identity::from(&ctx);
        let response = self.authenticate_barcode_blocking(identity, req);
        ctx.spawn(async {
            match response {
                Ok(r) => sink.success(r),
                Err(_) => sink.fail(RpcStatus::new(400)),
            }
            .map_err(move |e| error!("failed to reply authenticate_barcode: {:?}", e))
            .await
            .unwrap();
        })
    }

    fn authenticate_nfc_type(
        &mut self,
        ctx: grpcio::RpcContext,
        req: crate::grpc::authentication::AuthenticateNfcTypeRequest,
        sink: grpcio::UnarySink<crate::grpc::authentication::AuthenticateNfcTypeResponse>,
    ) {
        let identity = Identity::from(&ctx);
        let response = self.authenticate_nfc_type_blocking(identity, req);
        ctx.spawn(async {
            match response {
                Ok(r) => sink.success(r),
                Err(_) => sink.fail(RpcStatus::new(400)),
            }
            .map_err(move |e| error!("failed to reply authenticate_nfc_type: {:?}", e))
            .await
            .unwrap();
        })
    }

    fn authenticate_nfc_generic(
        &mut self,
        ctx: grpcio::RpcContext,
        req: crate::grpc::authentication::AuthenticateNfcGenericRequest,
        sink: grpcio::UnarySink<crate::grpc::authentication::AuthenticateNfcGenericResponse>,
    ) {
        let identity = Identity::from(&ctx);
        let response = self.authenticate_nfc_generic_blocking(identity, req);
        ctx.spawn(async {
            match response {
                Ok(r) => sink.success(r),
                Err(_) => sink.fail(RpcStatus::new(400)),
            }
            .map_err(move |e| error!("failed to reply authenticate_nfc_generic: {:?}", e))
            .await
            .unwrap();
        })
    }

    fn authenticate_nfc_mifare_desfire_phase1(
        &mut self,
        ctx: grpcio::RpcContext,
        req: crate::grpc::authentication::AuthenticateNfcMifareDesfirePhase1Request,
        sink: grpcio::UnarySink<
            crate::grpc::authentication::AuthenticateNfcMifareDesfirePhase1Response,
        >,
    ) {
        let identity = Identity::from(&ctx);
        let response = self.authenticate_nfc_mifare_desfire_phase1_blocking(identity, req);
        ctx.spawn(async {
            match response {
                Ok(r) => sink.success(r),
                Err(_) => sink.fail(RpcStatus::new(400)),
            }
            .map_err(move |e| {
                error!(
                    "failed to reply authenticate_nfc_mifare_desfire_phase1: {:?}",
                    e
                )
            })
            .await
            .unwrap();
        })
    }

    fn authenticate_nfc_mifare_desfire_phase2(
        &mut self,
        ctx: grpcio::RpcContext,
        req: crate::grpc::authentication::AuthenticateNfcMifareDesfirePhase2Request,
        sink: grpcio::UnarySink<
            crate::grpc::authentication::AuthenticateNfcMifareDesfirePhase2Response,
        >,
    ) {
        let identity = Identity::from(&ctx);
        let response = self.authenticate_nfc_mifare_desfire_phase2_blocking(identity, req);
        ctx.spawn(async {
            match response {
                Ok(r) => sink.success(r),
                Err(_) => sink.fail(RpcStatus::new(400)),
            }
            .map_err(move |e| {
                error!(
                    "failed to reply authenticate_nfc_mifare_desfire_phase2: {:?}",
                    e
                )
            })
            .await
            .unwrap();
        })
    }

    fn authenticate_nfc_generic_init_card(
        &mut self,
        ctx: grpcio::RpcContext,
        req: crate::grpc::authentication::AuthenticateNfcGenericInitCardRequest,
        sink: grpcio::UnarySink<
            crate::grpc::authentication::AuthenticateNfcGenericInitCardResponse,
        >,
    ) {
        let identity = Identity::from(&ctx);
        let response = self.authenticate_nfc_generic_init_card_blocking(identity, req);
        ctx.spawn(async {
            match response {
                Ok(r) => sink.success(r),
                Err(_) => sink.fail(RpcStatus::new(400)),
            }
            .map_err(move |e| {
                error!(
                    "failed to reply authenticate_nfc_generic_init_card: {:?}",
                    e
                )
            })
            .await
            .unwrap();
        })
    }

    fn authenticate_nfc_mifare_desfire_init_card(
        &mut self,
        ctx: grpcio::RpcContext,
        req: crate::grpc::authentication::AuthenticateNfcMifareDesfireInitCardRequest,
        sink: grpcio::UnarySink<
            crate::grpc::authentication::AuthenticateNfcMifareDesfireInitCardResponse,
        >,
    ) {
        let identity = Identity::from(&ctx);
        let response = self.authenticate_nfc_mifare_desfire_init_card_blocking(identity, req);
        ctx.spawn(async {
            match response {
                Ok(r) => sink.success(r),
                Err(e) => sink.fail(RpcStatus::with_message(500, e.to_string())),
            }
            .map_err(move |e| {
                error!(
                    "failed to reply authenticate_nfc_mifare_desfire_init_card: {:?}",
                    e
                )
            })
            .await
            .unwrap();
        })
    }
}

pub fn start_tcp_server(database_pool: DatabasePool, redis_pool: RedisPool) {
    let runner = AuthenticationRunner::new(database_pool, redis_pool);
    let send = runner.run();

    thread::spawn(move || {
        let env = Arc::new(Environment::new(1));
        let service = create_ascii_pay_authentication(AuthenticationImpl::new(send));

        let quota =
            ResourceQuota::new(Some("AuthenticationServerQuota")).resize_memory(1024 * 1024);
        let ch_builder = ChannelBuilder::new(env.clone()).set_resource_quota(quota);

        let mut server = ServerBuilder::new(env)
            .register_service(service)
            .bind(env::HOST.as_str(), *env::GRPC_PORT)
            .channel_args(ch_builder.build_args())
            .build()
            .expect("GRPC server could not be started!");
        server.start();
        for (host, port) in server.bind_addrs() {
            info!("listening on {}:{}", host, port);

            info!("Start grpc server at {}:{}", host, port);
        }

        loop {
            thread::park();
        }
    });
}
