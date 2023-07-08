# ascii-pay-server

## Run integration tests

Integration test are based on a postman collection and executed via `newman`. All test are run within `docker`/`docker compose` so no additional dependencies are necessary.

```bash
./tests.sh

# OR

docker compose up -d postgres
cargo run
cd collections
newman run ascii-pay-tests.postman_collection.json
cd ..
docker compose down
```

## apple wallet pass setup

![wallet pass strip](AsciiPayCard.pass/strip.png)

Requirements:
- `AppleWWDRCAG3.cer`
- `pass.cer` (from https://developer.apple.com/account/resources/certificates/list)
- `key.pem` (from CertificateSigningRequest for `pass.cer`)
  - if you have a `.p12` file after exporting the key from Keychain, the `.pem` file can be generated with:
    `openssl pkcs12 -in key.p12 -nodes -legacy -nomacver > key.pem`

```bash
openssl x509 -inform der -in AppleWWDRCAG3.cer -out apple-wwdr.pem
openssl x509 -inform der -in pass.cer -out apple-apns.pem
openssl pkcs12 -export -in pass.cer -inkey key.pem -out apple-pass.p12
```

## nfc authentication

```mermaid
sequenceDiagram
    participant card
    participant terminal
    participant ui
    participant server
    card->>terminal: put card on reader
    terminal->>+card: get static id
    card->>-terminal: get static id
    terminal->>ui: identify id
    ui->>+server: identify id
    server->>-ui: (ASCII_CARD / GENERIC_CARD / UNKNOWN)
    alt ASCII_CARD
        ui->>terminal: auth phase 1
        terminal->>+card: auth phase 1
        card->>-terminal: auth phase 1
        terminal->>ui: auth phase 1
        ui->>+server: auth phase 1
        server->>-ui: auth phase 1
        ui->>terminal: auth phase 2
        terminal->>+card: auth phase 2
        card->>-terminal: auth phase 2
        terminal->>ui: auth phase 2
        ui->>+server: auth phase 2
        server->>-ui: auth phase 2 <<session_token>>
    else GENERIC_CARD
        ui->>+terminal: auth phase 1
        terminal->>-ui: auth phase 1
        ui->>+server: auth phase 1
        server->>-ui: auth phase 1
        ui->>+terminal: auth phase 2
        terminal->>-ui: auth phase 2
        ui->>+server: auth phase 2
        server->>-ui: auth phase 2 <<session_token>>
    end
```

For `GENERIC_CARD`s the terminal contains a private key to perform the challenge response process.
