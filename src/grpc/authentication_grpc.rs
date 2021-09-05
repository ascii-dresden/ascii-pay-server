// This file is generated. Do not edit
// @generated

// https://github.com/Manishearth/rust-clippy/issues/702
#![allow(unknown_lints)]
#![allow(clippy::all)]

#![cfg_attr(rustfmt, rustfmt_skip)]

#![allow(box_pointers)]
#![allow(dead_code)]
#![allow(missing_docs)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(trivial_casts)]
#![allow(unsafe_code)]
#![allow(unused_imports)]
#![allow(unused_results)]


// server interface

pub trait AsciiPayAuthentication {
    fn authenticate_barcode(&self, o: ::grpc::ServerHandlerContext, req: ::grpc::ServerRequestSingle<super::authentication::AuthenticateBarcodeRequest>, resp: ::grpc::ServerResponseUnarySink<super::authentication::AuthenticateBarcodeResponse>) -> ::grpc::Result<()>;

    fn authenticate_nfc_type(&self, o: ::grpc::ServerHandlerContext, req: ::grpc::ServerRequestSingle<super::authentication::AuthenticateNfcTypeRequest>, resp: ::grpc::ServerResponseUnarySink<super::authentication::AuthenticateNfcTypeResponse>) -> ::grpc::Result<()>;

    fn authenticate_nfc_generic(&self, o: ::grpc::ServerHandlerContext, req: ::grpc::ServerRequestSingle<super::authentication::AuthenticateNfcGenericRequest>, resp: ::grpc::ServerResponseUnarySink<super::authentication::AuthenticateNfcGenericResponse>) -> ::grpc::Result<()>;

    fn authenticate_nfc_mifare_desfire_phase1(&self, o: ::grpc::ServerHandlerContext, req: ::grpc::ServerRequestSingle<super::authentication::AuthenticateNfcMifareDesfirePhase1Request>, resp: ::grpc::ServerResponseUnarySink<super::authentication::AuthenticateNfcMifareDesfirePhase1Response>) -> ::grpc::Result<()>;

    fn authenticate_nfc_mifare_desfire_phase2(&self, o: ::grpc::ServerHandlerContext, req: ::grpc::ServerRequestSingle<super::authentication::AuthenticateNfcMifareDesfirePhase2Request>, resp: ::grpc::ServerResponseUnarySink<super::authentication::AuthenticateNfcMifareDesfirePhase2Response>) -> ::grpc::Result<()>;

    fn authenticate_nfc_generic_init_card(&self, o: ::grpc::ServerHandlerContext, req: ::grpc::ServerRequestSingle<super::authentication::AuthenticateNfcGenericInitCardRequest>, resp: ::grpc::ServerResponseUnarySink<super::authentication::AuthenticateNfcGenericInitCardResponse>) -> ::grpc::Result<()>;

    fn authenticate_nfc_mifare_desfire_init_card(&self, o: ::grpc::ServerHandlerContext, req: ::grpc::ServerRequestSingle<super::authentication::AuthenticateNfcMifareDesfireInitCardRequest>, resp: ::grpc::ServerResponseUnarySink<super::authentication::AuthenticateNfcMifareDesfireInitCardResponse>) -> ::grpc::Result<()>;
}

// client

pub struct AsciiPayAuthenticationClient {
    grpc_client: ::std::sync::Arc<::grpc::Client>,
}

impl ::grpc::ClientStub for AsciiPayAuthenticationClient {
    fn with_client(grpc_client: ::std::sync::Arc<::grpc::Client>) -> Self {
        AsciiPayAuthenticationClient {
            grpc_client: grpc_client,
        }
    }
}

impl AsciiPayAuthenticationClient {
    pub fn authenticate_barcode(&self, o: ::grpc::RequestOptions, req: super::authentication::AuthenticateBarcodeRequest) -> ::grpc::SingleResponse<super::authentication::AuthenticateBarcodeResponse> {
        let descriptor = ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
            name: ::grpc::rt::StringOrStatic::Static("/authentication.AsciiPayAuthentication/AuthenticateBarcode"),
            streaming: ::grpc::rt::GrpcStreaming::Unary,
            req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
            resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
        });
        self.grpc_client.call_unary(o, req, descriptor)
    }

    pub fn authenticate_nfc_type(&self, o: ::grpc::RequestOptions, req: super::authentication::AuthenticateNfcTypeRequest) -> ::grpc::SingleResponse<super::authentication::AuthenticateNfcTypeResponse> {
        let descriptor = ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
            name: ::grpc::rt::StringOrStatic::Static("/authentication.AsciiPayAuthentication/AuthenticateNfcType"),
            streaming: ::grpc::rt::GrpcStreaming::Unary,
            req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
            resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
        });
        self.grpc_client.call_unary(o, req, descriptor)
    }

    pub fn authenticate_nfc_generic(&self, o: ::grpc::RequestOptions, req: super::authentication::AuthenticateNfcGenericRequest) -> ::grpc::SingleResponse<super::authentication::AuthenticateNfcGenericResponse> {
        let descriptor = ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
            name: ::grpc::rt::StringOrStatic::Static("/authentication.AsciiPayAuthentication/AuthenticateNfcGeneric"),
            streaming: ::grpc::rt::GrpcStreaming::Unary,
            req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
            resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
        });
        self.grpc_client.call_unary(o, req, descriptor)
    }

    pub fn authenticate_nfc_mifare_desfire_phase1(&self, o: ::grpc::RequestOptions, req: super::authentication::AuthenticateNfcMifareDesfirePhase1Request) -> ::grpc::SingleResponse<super::authentication::AuthenticateNfcMifareDesfirePhase1Response> {
        let descriptor = ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
            name: ::grpc::rt::StringOrStatic::Static("/authentication.AsciiPayAuthentication/AuthenticateNfcMifareDesfirePhase1"),
            streaming: ::grpc::rt::GrpcStreaming::Unary,
            req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
            resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
        });
        self.grpc_client.call_unary(o, req, descriptor)
    }

    pub fn authenticate_nfc_mifare_desfire_phase2(&self, o: ::grpc::RequestOptions, req: super::authentication::AuthenticateNfcMifareDesfirePhase2Request) -> ::grpc::SingleResponse<super::authentication::AuthenticateNfcMifareDesfirePhase2Response> {
        let descriptor = ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
            name: ::grpc::rt::StringOrStatic::Static("/authentication.AsciiPayAuthentication/AuthenticateNfcMifareDesfirePhase2"),
            streaming: ::grpc::rt::GrpcStreaming::Unary,
            req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
            resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
        });
        self.grpc_client.call_unary(o, req, descriptor)
    }

    pub fn authenticate_nfc_generic_init_card(&self, o: ::grpc::RequestOptions, req: super::authentication::AuthenticateNfcGenericInitCardRequest) -> ::grpc::SingleResponse<super::authentication::AuthenticateNfcGenericInitCardResponse> {
        let descriptor = ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
            name: ::grpc::rt::StringOrStatic::Static("/authentication.AsciiPayAuthentication/AuthenticateNfcGenericInitCard"),
            streaming: ::grpc::rt::GrpcStreaming::Unary,
            req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
            resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
        });
        self.grpc_client.call_unary(o, req, descriptor)
    }

    pub fn authenticate_nfc_mifare_desfire_init_card(&self, o: ::grpc::RequestOptions, req: super::authentication::AuthenticateNfcMifareDesfireInitCardRequest) -> ::grpc::SingleResponse<super::authentication::AuthenticateNfcMifareDesfireInitCardResponse> {
        let descriptor = ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
            name: ::grpc::rt::StringOrStatic::Static("/authentication.AsciiPayAuthentication/AuthenticateNfcMifareDesfireInitCard"),
            streaming: ::grpc::rt::GrpcStreaming::Unary,
            req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
            resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
        });
        self.grpc_client.call_unary(o, req, descriptor)
    }
}

// server

pub struct AsciiPayAuthenticationServer;


impl AsciiPayAuthenticationServer {
    pub fn new_service_def<H : AsciiPayAuthentication + 'static + Sync + Send + 'static>(handler: H) -> ::grpc::rt::ServerServiceDefinition {
        let handler_arc = ::std::sync::Arc::new(handler);
        ::grpc::rt::ServerServiceDefinition::new("/authentication.AsciiPayAuthentication",
            vec![
                ::grpc::rt::ServerMethod::new(
                    ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
                        name: ::grpc::rt::StringOrStatic::Static("/authentication.AsciiPayAuthentication/AuthenticateBarcode"),
                        streaming: ::grpc::rt::GrpcStreaming::Unary,
                        req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                        resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerUnary::new(move |ctx, req, resp| (*handler_copy).authenticate_barcode(ctx, req, resp))
                    },
                ),
                ::grpc::rt::ServerMethod::new(
                    ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
                        name: ::grpc::rt::StringOrStatic::Static("/authentication.AsciiPayAuthentication/AuthenticateNfcType"),
                        streaming: ::grpc::rt::GrpcStreaming::Unary,
                        req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                        resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerUnary::new(move |ctx, req, resp| (*handler_copy).authenticate_nfc_type(ctx, req, resp))
                    },
                ),
                ::grpc::rt::ServerMethod::new(
                    ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
                        name: ::grpc::rt::StringOrStatic::Static("/authentication.AsciiPayAuthentication/AuthenticateNfcGeneric"),
                        streaming: ::grpc::rt::GrpcStreaming::Unary,
                        req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                        resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerUnary::new(move |ctx, req, resp| (*handler_copy).authenticate_nfc_generic(ctx, req, resp))
                    },
                ),
                ::grpc::rt::ServerMethod::new(
                    ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
                        name: ::grpc::rt::StringOrStatic::Static("/authentication.AsciiPayAuthentication/AuthenticateNfcMifareDesfirePhase1"),
                        streaming: ::grpc::rt::GrpcStreaming::Unary,
                        req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                        resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerUnary::new(move |ctx, req, resp| (*handler_copy).authenticate_nfc_mifare_desfire_phase1(ctx, req, resp))
                    },
                ),
                ::grpc::rt::ServerMethod::new(
                    ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
                        name: ::grpc::rt::StringOrStatic::Static("/authentication.AsciiPayAuthentication/AuthenticateNfcMifareDesfirePhase2"),
                        streaming: ::grpc::rt::GrpcStreaming::Unary,
                        req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                        resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerUnary::new(move |ctx, req, resp| (*handler_copy).authenticate_nfc_mifare_desfire_phase2(ctx, req, resp))
                    },
                ),
                ::grpc::rt::ServerMethod::new(
                    ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
                        name: ::grpc::rt::StringOrStatic::Static("/authentication.AsciiPayAuthentication/AuthenticateNfcGenericInitCard"),
                        streaming: ::grpc::rt::GrpcStreaming::Unary,
                        req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                        resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerUnary::new(move |ctx, req, resp| (*handler_copy).authenticate_nfc_generic_init_card(ctx, req, resp))
                    },
                ),
                ::grpc::rt::ServerMethod::new(
                    ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
                        name: ::grpc::rt::StringOrStatic::Static("/authentication.AsciiPayAuthentication/AuthenticateNfcMifareDesfireInitCard"),
                        streaming: ::grpc::rt::GrpcStreaming::Unary,
                        req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                        resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerUnary::new(move |ctx, req, resp| (*handler_copy).authenticate_nfc_mifare_desfire_init_card(ctx, req, resp))
                    },
                ),
            ],
        )
    }
}
