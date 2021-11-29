// This file is generated. Do not edit
// @generated

// https://github.com/Manishearth/rust-clippy/issues/702
#![allow(unknown_lints)]
#![allow(clippy::all)]
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

const METHOD_ASCII_PAY_AUTHENTICATION_AUTHENTICATE_BARCODE: ::grpcio::Method<
    super::authentication::AuthenticateBarcodeRequest,
    super::authentication::AuthenticateBarcodeResponse,
> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/authentication.AsciiPayAuthentication/AuthenticateBarcode",
    req_mar: ::grpcio::Marshaller {
        ser: ::grpcio::pb_ser,
        de: ::grpcio::pb_de,
    },
    resp_mar: ::grpcio::Marshaller {
        ser: ::grpcio::pb_ser,
        de: ::grpcio::pb_de,
    },
};

const METHOD_ASCII_PAY_AUTHENTICATION_AUTHENTICATE_NFC_TYPE: ::grpcio::Method<
    super::authentication::AuthenticateNfcTypeRequest,
    super::authentication::AuthenticateNfcTypeResponse,
> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/authentication.AsciiPayAuthentication/AuthenticateNfcType",
    req_mar: ::grpcio::Marshaller {
        ser: ::grpcio::pb_ser,
        de: ::grpcio::pb_de,
    },
    resp_mar: ::grpcio::Marshaller {
        ser: ::grpcio::pb_ser,
        de: ::grpcio::pb_de,
    },
};

const METHOD_ASCII_PAY_AUTHENTICATION_AUTHENTICATE_NFC_GENERIC: ::grpcio::Method<
    super::authentication::AuthenticateNfcGenericRequest,
    super::authentication::AuthenticateNfcGenericResponse,
> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/authentication.AsciiPayAuthentication/AuthenticateNfcGeneric",
    req_mar: ::grpcio::Marshaller {
        ser: ::grpcio::pb_ser,
        de: ::grpcio::pb_de,
    },
    resp_mar: ::grpcio::Marshaller {
        ser: ::grpcio::pb_ser,
        de: ::grpcio::pb_de,
    },
};

const METHOD_ASCII_PAY_AUTHENTICATION_AUTHENTICATE_NFC_MIFARE_DESFIRE_PHASE1: ::grpcio::Method<
    super::authentication::AuthenticateNfcMifareDesfirePhase1Request,
    super::authentication::AuthenticateNfcMifareDesfirePhase1Response,
> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/authentication.AsciiPayAuthentication/AuthenticateNfcMifareDesfirePhase1",
    req_mar: ::grpcio::Marshaller {
        ser: ::grpcio::pb_ser,
        de: ::grpcio::pb_de,
    },
    resp_mar: ::grpcio::Marshaller {
        ser: ::grpcio::pb_ser,
        de: ::grpcio::pb_de,
    },
};

const METHOD_ASCII_PAY_AUTHENTICATION_AUTHENTICATE_NFC_MIFARE_DESFIRE_PHASE2: ::grpcio::Method<
    super::authentication::AuthenticateNfcMifareDesfirePhase2Request,
    super::authentication::AuthenticateNfcMifareDesfirePhase2Response,
> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/authentication.AsciiPayAuthentication/AuthenticateNfcMifareDesfirePhase2",
    req_mar: ::grpcio::Marshaller {
        ser: ::grpcio::pb_ser,
        de: ::grpcio::pb_de,
    },
    resp_mar: ::grpcio::Marshaller {
        ser: ::grpcio::pb_ser,
        de: ::grpcio::pb_de,
    },
};

const METHOD_ASCII_PAY_AUTHENTICATION_AUTHENTICATE_NFC_GENERIC_INIT_CARD: ::grpcio::Method<
    super::authentication::AuthenticateNfcGenericInitCardRequest,
    super::authentication::AuthenticateNfcGenericInitCardResponse,
> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/authentication.AsciiPayAuthentication/AuthenticateNfcGenericInitCard",
    req_mar: ::grpcio::Marshaller {
        ser: ::grpcio::pb_ser,
        de: ::grpcio::pb_de,
    },
    resp_mar: ::grpcio::Marshaller {
        ser: ::grpcio::pb_ser,
        de: ::grpcio::pb_de,
    },
};

const METHOD_ASCII_PAY_AUTHENTICATION_AUTHENTICATE_NFC_MIFARE_DESFIRE_INIT_CARD: ::grpcio::Method<
    super::authentication::AuthenticateNfcMifareDesfireInitCardRequest,
    super::authentication::AuthenticateNfcMifareDesfireInitCardResponse,
> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/authentication.AsciiPayAuthentication/AuthenticateNfcMifareDesfireInitCard",
    req_mar: ::grpcio::Marshaller {
        ser: ::grpcio::pb_ser,
        de: ::grpcio::pb_de,
    },
    resp_mar: ::grpcio::Marshaller {
        ser: ::grpcio::pb_ser,
        de: ::grpcio::pb_de,
    },
};

#[derive(Clone)]
pub struct AsciiPayAuthenticationClient {
    client: ::grpcio::Client,
}

impl AsciiPayAuthenticationClient {
    pub fn new(channel: ::grpcio::Channel) -> Self {
        AsciiPayAuthenticationClient {
            client: ::grpcio::Client::new(channel),
        }
    }

    pub fn authenticate_barcode_opt(
        &self,
        req: &super::authentication::AuthenticateBarcodeRequest,
        opt: ::grpcio::CallOption,
    ) -> ::grpcio::Result<super::authentication::AuthenticateBarcodeResponse> {
        self.client.unary_call(
            &METHOD_ASCII_PAY_AUTHENTICATION_AUTHENTICATE_BARCODE,
            req,
            opt,
        )
    }

    pub fn authenticate_barcode(
        &self,
        req: &super::authentication::AuthenticateBarcodeRequest,
    ) -> ::grpcio::Result<super::authentication::AuthenticateBarcodeResponse> {
        self.authenticate_barcode_opt(req, ::grpcio::CallOption::default())
    }

    pub fn authenticate_barcode_async_opt(
        &self,
        req: &super::authentication::AuthenticateBarcodeRequest,
        opt: ::grpcio::CallOption,
    ) -> ::grpcio::Result<
        ::grpcio::ClientUnaryReceiver<super::authentication::AuthenticateBarcodeResponse>,
    > {
        self.client.unary_call_async(
            &METHOD_ASCII_PAY_AUTHENTICATION_AUTHENTICATE_BARCODE,
            req,
            opt,
        )
    }

    pub fn authenticate_barcode_async(
        &self,
        req: &super::authentication::AuthenticateBarcodeRequest,
    ) -> ::grpcio::Result<
        ::grpcio::ClientUnaryReceiver<super::authentication::AuthenticateBarcodeResponse>,
    > {
        self.authenticate_barcode_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn authenticate_nfc_type_opt(
        &self,
        req: &super::authentication::AuthenticateNfcTypeRequest,
        opt: ::grpcio::CallOption,
    ) -> ::grpcio::Result<super::authentication::AuthenticateNfcTypeResponse> {
        self.client.unary_call(
            &METHOD_ASCII_PAY_AUTHENTICATION_AUTHENTICATE_NFC_TYPE,
            req,
            opt,
        )
    }

    pub fn authenticate_nfc_type(
        &self,
        req: &super::authentication::AuthenticateNfcTypeRequest,
    ) -> ::grpcio::Result<super::authentication::AuthenticateNfcTypeResponse> {
        self.authenticate_nfc_type_opt(req, ::grpcio::CallOption::default())
    }

    pub fn authenticate_nfc_type_async_opt(
        &self,
        req: &super::authentication::AuthenticateNfcTypeRequest,
        opt: ::grpcio::CallOption,
    ) -> ::grpcio::Result<
        ::grpcio::ClientUnaryReceiver<super::authentication::AuthenticateNfcTypeResponse>,
    > {
        self.client.unary_call_async(
            &METHOD_ASCII_PAY_AUTHENTICATION_AUTHENTICATE_NFC_TYPE,
            req,
            opt,
        )
    }

    pub fn authenticate_nfc_type_async(
        &self,
        req: &super::authentication::AuthenticateNfcTypeRequest,
    ) -> ::grpcio::Result<
        ::grpcio::ClientUnaryReceiver<super::authentication::AuthenticateNfcTypeResponse>,
    > {
        self.authenticate_nfc_type_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn authenticate_nfc_generic_opt(
        &self,
        req: &super::authentication::AuthenticateNfcGenericRequest,
        opt: ::grpcio::CallOption,
    ) -> ::grpcio::Result<super::authentication::AuthenticateNfcGenericResponse> {
        self.client.unary_call(
            &METHOD_ASCII_PAY_AUTHENTICATION_AUTHENTICATE_NFC_GENERIC,
            req,
            opt,
        )
    }

    pub fn authenticate_nfc_generic(
        &self,
        req: &super::authentication::AuthenticateNfcGenericRequest,
    ) -> ::grpcio::Result<super::authentication::AuthenticateNfcGenericResponse> {
        self.authenticate_nfc_generic_opt(req, ::grpcio::CallOption::default())
    }

    pub fn authenticate_nfc_generic_async_opt(
        &self,
        req: &super::authentication::AuthenticateNfcGenericRequest,
        opt: ::grpcio::CallOption,
    ) -> ::grpcio::Result<
        ::grpcio::ClientUnaryReceiver<super::authentication::AuthenticateNfcGenericResponse>,
    > {
        self.client.unary_call_async(
            &METHOD_ASCII_PAY_AUTHENTICATION_AUTHENTICATE_NFC_GENERIC,
            req,
            opt,
        )
    }

    pub fn authenticate_nfc_generic_async(
        &self,
        req: &super::authentication::AuthenticateNfcGenericRequest,
    ) -> ::grpcio::Result<
        ::grpcio::ClientUnaryReceiver<super::authentication::AuthenticateNfcGenericResponse>,
    > {
        self.authenticate_nfc_generic_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn authenticate_nfc_mifare_desfire_phase1_opt(
        &self,
        req: &super::authentication::AuthenticateNfcMifareDesfirePhase1Request,
        opt: ::grpcio::CallOption,
    ) -> ::grpcio::Result<super::authentication::AuthenticateNfcMifareDesfirePhase1Response> {
        self.client.unary_call(
            &METHOD_ASCII_PAY_AUTHENTICATION_AUTHENTICATE_NFC_MIFARE_DESFIRE_PHASE1,
            req,
            opt,
        )
    }

    pub fn authenticate_nfc_mifare_desfire_phase1(
        &self,
        req: &super::authentication::AuthenticateNfcMifareDesfirePhase1Request,
    ) -> ::grpcio::Result<super::authentication::AuthenticateNfcMifareDesfirePhase1Response> {
        self.authenticate_nfc_mifare_desfire_phase1_opt(req, ::grpcio::CallOption::default())
    }

    pub fn authenticate_nfc_mifare_desfire_phase1_async_opt(
        &self,
        req: &super::authentication::AuthenticateNfcMifareDesfirePhase1Request,
        opt: ::grpcio::CallOption,
    ) -> ::grpcio::Result<
        ::grpcio::ClientUnaryReceiver<
            super::authentication::AuthenticateNfcMifareDesfirePhase1Response,
        >,
    > {
        self.client.unary_call_async(
            &METHOD_ASCII_PAY_AUTHENTICATION_AUTHENTICATE_NFC_MIFARE_DESFIRE_PHASE1,
            req,
            opt,
        )
    }

    pub fn authenticate_nfc_mifare_desfire_phase1_async(
        &self,
        req: &super::authentication::AuthenticateNfcMifareDesfirePhase1Request,
    ) -> ::grpcio::Result<
        ::grpcio::ClientUnaryReceiver<
            super::authentication::AuthenticateNfcMifareDesfirePhase1Response,
        >,
    > {
        self.authenticate_nfc_mifare_desfire_phase1_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn authenticate_nfc_mifare_desfire_phase2_opt(
        &self,
        req: &super::authentication::AuthenticateNfcMifareDesfirePhase2Request,
        opt: ::grpcio::CallOption,
    ) -> ::grpcio::Result<super::authentication::AuthenticateNfcMifareDesfirePhase2Response> {
        self.client.unary_call(
            &METHOD_ASCII_PAY_AUTHENTICATION_AUTHENTICATE_NFC_MIFARE_DESFIRE_PHASE2,
            req,
            opt,
        )
    }

    pub fn authenticate_nfc_mifare_desfire_phase2(
        &self,
        req: &super::authentication::AuthenticateNfcMifareDesfirePhase2Request,
    ) -> ::grpcio::Result<super::authentication::AuthenticateNfcMifareDesfirePhase2Response> {
        self.authenticate_nfc_mifare_desfire_phase2_opt(req, ::grpcio::CallOption::default())
    }

    pub fn authenticate_nfc_mifare_desfire_phase2_async_opt(
        &self,
        req: &super::authentication::AuthenticateNfcMifareDesfirePhase2Request,
        opt: ::grpcio::CallOption,
    ) -> ::grpcio::Result<
        ::grpcio::ClientUnaryReceiver<
            super::authentication::AuthenticateNfcMifareDesfirePhase2Response,
        >,
    > {
        self.client.unary_call_async(
            &METHOD_ASCII_PAY_AUTHENTICATION_AUTHENTICATE_NFC_MIFARE_DESFIRE_PHASE2,
            req,
            opt,
        )
    }

    pub fn authenticate_nfc_mifare_desfire_phase2_async(
        &self,
        req: &super::authentication::AuthenticateNfcMifareDesfirePhase2Request,
    ) -> ::grpcio::Result<
        ::grpcio::ClientUnaryReceiver<
            super::authentication::AuthenticateNfcMifareDesfirePhase2Response,
        >,
    > {
        self.authenticate_nfc_mifare_desfire_phase2_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn authenticate_nfc_generic_init_card_opt(
        &self,
        req: &super::authentication::AuthenticateNfcGenericInitCardRequest,
        opt: ::grpcio::CallOption,
    ) -> ::grpcio::Result<super::authentication::AuthenticateNfcGenericInitCardResponse> {
        self.client.unary_call(
            &METHOD_ASCII_PAY_AUTHENTICATION_AUTHENTICATE_NFC_GENERIC_INIT_CARD,
            req,
            opt,
        )
    }

    pub fn authenticate_nfc_generic_init_card(
        &self,
        req: &super::authentication::AuthenticateNfcGenericInitCardRequest,
    ) -> ::grpcio::Result<super::authentication::AuthenticateNfcGenericInitCardResponse> {
        self.authenticate_nfc_generic_init_card_opt(req, ::grpcio::CallOption::default())
    }

    pub fn authenticate_nfc_generic_init_card_async_opt(
        &self,
        req: &super::authentication::AuthenticateNfcGenericInitCardRequest,
        opt: ::grpcio::CallOption,
    ) -> ::grpcio::Result<
        ::grpcio::ClientUnaryReceiver<
            super::authentication::AuthenticateNfcGenericInitCardResponse,
        >,
    > {
        self.client.unary_call_async(
            &METHOD_ASCII_PAY_AUTHENTICATION_AUTHENTICATE_NFC_GENERIC_INIT_CARD,
            req,
            opt,
        )
    }

    pub fn authenticate_nfc_generic_init_card_async(
        &self,
        req: &super::authentication::AuthenticateNfcGenericInitCardRequest,
    ) -> ::grpcio::Result<
        ::grpcio::ClientUnaryReceiver<
            super::authentication::AuthenticateNfcGenericInitCardResponse,
        >,
    > {
        self.authenticate_nfc_generic_init_card_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn authenticate_nfc_mifare_desfire_init_card_opt(
        &self,
        req: &super::authentication::AuthenticateNfcMifareDesfireInitCardRequest,
        opt: ::grpcio::CallOption,
    ) -> ::grpcio::Result<super::authentication::AuthenticateNfcMifareDesfireInitCardResponse> {
        self.client.unary_call(
            &METHOD_ASCII_PAY_AUTHENTICATION_AUTHENTICATE_NFC_MIFARE_DESFIRE_INIT_CARD,
            req,
            opt,
        )
    }

    pub fn authenticate_nfc_mifare_desfire_init_card(
        &self,
        req: &super::authentication::AuthenticateNfcMifareDesfireInitCardRequest,
    ) -> ::grpcio::Result<super::authentication::AuthenticateNfcMifareDesfireInitCardResponse> {
        self.authenticate_nfc_mifare_desfire_init_card_opt(req, ::grpcio::CallOption::default())
    }

    pub fn authenticate_nfc_mifare_desfire_init_card_async_opt(
        &self,
        req: &super::authentication::AuthenticateNfcMifareDesfireInitCardRequest,
        opt: ::grpcio::CallOption,
    ) -> ::grpcio::Result<
        ::grpcio::ClientUnaryReceiver<
            super::authentication::AuthenticateNfcMifareDesfireInitCardResponse,
        >,
    > {
        self.client.unary_call_async(
            &METHOD_ASCII_PAY_AUTHENTICATION_AUTHENTICATE_NFC_MIFARE_DESFIRE_INIT_CARD,
            req,
            opt,
        )
    }

    pub fn authenticate_nfc_mifare_desfire_init_card_async(
        &self,
        req: &super::authentication::AuthenticateNfcMifareDesfireInitCardRequest,
    ) -> ::grpcio::Result<
        ::grpcio::ClientUnaryReceiver<
            super::authentication::AuthenticateNfcMifareDesfireInitCardResponse,
        >,
    > {
        self.authenticate_nfc_mifare_desfire_init_card_async_opt(
            req,
            ::grpcio::CallOption::default(),
        )
    }
    pub fn spawn<F>(&self, f: F)
    where
        F: ::futures::Future<Output = ()> + Send + 'static,
    {
        self.client.spawn(f)
    }
}

pub trait AsciiPayAuthentication {
    fn authenticate_barcode(
        &mut self,
        ctx: ::grpcio::RpcContext,
        req: super::authentication::AuthenticateBarcodeRequest,
        sink: ::grpcio::UnarySink<super::authentication::AuthenticateBarcodeResponse>,
    );
    fn authenticate_nfc_type(
        &mut self,
        ctx: ::grpcio::RpcContext,
        req: super::authentication::AuthenticateNfcTypeRequest,
        sink: ::grpcio::UnarySink<super::authentication::AuthenticateNfcTypeResponse>,
    );
    fn authenticate_nfc_generic(
        &mut self,
        ctx: ::grpcio::RpcContext,
        req: super::authentication::AuthenticateNfcGenericRequest,
        sink: ::grpcio::UnarySink<super::authentication::AuthenticateNfcGenericResponse>,
    );
    fn authenticate_nfc_mifare_desfire_phase1(
        &mut self,
        ctx: ::grpcio::RpcContext,
        req: super::authentication::AuthenticateNfcMifareDesfirePhase1Request,
        sink: ::grpcio::UnarySink<
            super::authentication::AuthenticateNfcMifareDesfirePhase1Response,
        >,
    );
    fn authenticate_nfc_mifare_desfire_phase2(
        &mut self,
        ctx: ::grpcio::RpcContext,
        req: super::authentication::AuthenticateNfcMifareDesfirePhase2Request,
        sink: ::grpcio::UnarySink<
            super::authentication::AuthenticateNfcMifareDesfirePhase2Response,
        >,
    );
    fn authenticate_nfc_generic_init_card(
        &mut self,
        ctx: ::grpcio::RpcContext,
        req: super::authentication::AuthenticateNfcGenericInitCardRequest,
        sink: ::grpcio::UnarySink<super::authentication::AuthenticateNfcGenericInitCardResponse>,
    );
    fn authenticate_nfc_mifare_desfire_init_card(
        &mut self,
        ctx: ::grpcio::RpcContext,
        req: super::authentication::AuthenticateNfcMifareDesfireInitCardRequest,
        sink: ::grpcio::UnarySink<
            super::authentication::AuthenticateNfcMifareDesfireInitCardResponse,
        >,
    );
}

pub fn create_ascii_pay_authentication<S: AsciiPayAuthentication + Send + Clone + 'static>(
    s: S,
) -> ::grpcio::Service {
    let mut builder = ::grpcio::ServiceBuilder::new();
    let mut instance = s.clone();
    builder = builder.add_unary_handler(
        &METHOD_ASCII_PAY_AUTHENTICATION_AUTHENTICATE_BARCODE,
        move |ctx, req, resp| instance.authenticate_barcode(ctx, req, resp),
    );
    let mut instance = s.clone();
    builder = builder.add_unary_handler(
        &METHOD_ASCII_PAY_AUTHENTICATION_AUTHENTICATE_NFC_TYPE,
        move |ctx, req, resp| instance.authenticate_nfc_type(ctx, req, resp),
    );
    let mut instance = s.clone();
    builder = builder.add_unary_handler(
        &METHOD_ASCII_PAY_AUTHENTICATION_AUTHENTICATE_NFC_GENERIC,
        move |ctx, req, resp| instance.authenticate_nfc_generic(ctx, req, resp),
    );
    let mut instance = s.clone();
    builder = builder.add_unary_handler(
        &METHOD_ASCII_PAY_AUTHENTICATION_AUTHENTICATE_NFC_MIFARE_DESFIRE_PHASE1,
        move |ctx, req, resp| instance.authenticate_nfc_mifare_desfire_phase1(ctx, req, resp),
    );
    let mut instance = s.clone();
    builder = builder.add_unary_handler(
        &METHOD_ASCII_PAY_AUTHENTICATION_AUTHENTICATE_NFC_MIFARE_DESFIRE_PHASE2,
        move |ctx, req, resp| instance.authenticate_nfc_mifare_desfire_phase2(ctx, req, resp),
    );
    let mut instance = s.clone();
    builder = builder.add_unary_handler(
        &METHOD_ASCII_PAY_AUTHENTICATION_AUTHENTICATE_NFC_GENERIC_INIT_CARD,
        move |ctx, req, resp| instance.authenticate_nfc_generic_init_card(ctx, req, resp),
    );
    let mut instance = s;
    builder = builder.add_unary_handler(
        &METHOD_ASCII_PAY_AUTHENTICATION_AUTHENTICATE_NFC_MIFARE_DESFIRE_INIT_CARD,
        move |ctx, req, resp| instance.authenticate_nfc_mifare_desfire_init_card(ctx, req, resp),
    );
    builder.build()
}
