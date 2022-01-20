#!/bin/bash

rm -rf test-certificates/
mkdir test-certificates/
cd test-certificates/

openssl ecparam -out ascii-pay-root.key -name prime256v1 -genkey
openssl req -new -sha256 -key ascii-pay-root.key -out ascii-pay-root.csr -subj "/C=de/CN=ascii.local"
openssl x509 -req -sha256 -days 365 -in ascii-pay-root.csr -signkey ascii-pay-root.key -out ascii-pay-root.crt

openssl ecparam -out pay.ascii.local.key -name prime256v1 -genkey
openssl req -new -sha256 -key pay.ascii.local.key -out pay.ascii.local.csr -subj "/C=de/CN=pay.ascii.local"
openssl x509 -req -in pay.ascii.local.csr -CA  ascii-pay-root.crt -CAkey ascii-pay-root.key -CAcreateserial -out pay.ascii.local.crt -days 365 -sha256
openssl x509 -in pay.ascii.local.crt -text -noout

openssl ecparam -out secure-pay.ascii.local.key -name prime256v1 -genkey
openssl req -new -sha256 -key secure-pay.ascii.local.key -out secure-pay.ascii.local.csr -subj "/C=de/CN=secure-pay.ascii.local"
openssl x509 -req -in secure-pay.ascii.local.csr -CA  ascii-pay-root.crt -CAkey ascii-pay-root.key -CAcreateserial -out secure-pay.ascii.local.crt -days 365 -sha256
openssl x509 -in secure-pay.ascii.local.crt -text -noout
