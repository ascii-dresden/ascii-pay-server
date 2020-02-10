function parseSSE(data) {
    if (data && data.type) {
        switch (data.type) {
            case "account":
                load_account(data.content);
                break;
            case "product":
                load_product(data.content);
                break;
            case "qr-code":
                load_qr_code(data.content);
                break;
            case "nfc-card":
                load_nfc_card(data.content);
                break;
            case "remove-nfc-card":
                load_remove_nfc_card();
                break;
            case "payment-token":
                load_payment_token(data.content);
                break;
            case "timeout":
                load_timeout();
                break;
        }
    }
}

function hide_cards() {
    for (let card of document.getElementsByClassName("card-hidden")) {
        card.classList.remove("active");
    }
}

function load_account(content) {
    hide_cards();
    document.getElementById("card-account").classList.add("active");

    document.getElementById("card-account-name").value = content.name;
    document.getElementById("card-account-credit").value = (content.credit / 100).toFixed(2);
}

function load_product(content) {
    hide_cards();
    document.getElementById("card-product").classList.add("active");

    document.getElementById("card-product-name").value = content.name;
    document.getElementById("card-product-price").value = (content.current_price / 100).toFixed(2);
}

function load_qr_code(content) {
    hide_cards();
    document.getElementById("card-qr").classList.add("active");

    document.getElementById("card-qr-code").value = content.code;
}

function load_nfc_card(content) {
    hide_cards();
    document.getElementById("card-nfc").classList.add("active");

    document.getElementById("card-nfc-id").value = content.id;
    document.getElementById("card-nfc-name").value = content.name;
    document.getElementById("card-nfc-writeable").value = content.writeable;
}

function load_remove_nfc_card() {
    hide_cards();
}

function load_payment_token(content) {
    let token = content.token;
    let amount = Math.round(parseFloat(document.getElementById("card-payment-amount").value) * 100);

    let btn = document.getElementById("card-payment-pay");

    fetch("/api/v1/transaction/payment", {
        method: "POST",
        headers: {
            'Accept': 'application/json',
            'Content-Type': 'application/json'
        },
        body: JSON.stringify({
            amount: amount,
            token: token,
            products: {}
        })
    }).then((response) => {
        btn.classList.remove("loading");

        if (response.status === 200) {
            btn.classList.add("btn-success");

            setTimeout(() => {
                btn.classList.remove("btn-success");
            }, 2000);

            response.json().then((response) => {
                load_account(response.account);
            });
        } else {
            btn.classList.add("btn-error");

            setTimeout(() => {
                btn.classList.remove("btn-error");
            }, 2000);
        }

        fetch("/reauthenticate", {
            method: "GET",
            headers: {
                'Accept': 'application/json',
                'Content-Type': 'application/json'
            }
        });
    }).catch((reason) => {
        btn.classList.remove("loading");
        btn.classList.add("btn-error");

        setTimeout(() => {
            btn.classList.remove("btn-error");
        }, 2000);
    });
}

function load_timeout() {
    let btn = document.getElementById("card-payment-pay");
    
    btn.classList.remove("loading");
    btn.classList.add("btn-error");

    setTimeout(() => {
        btn.classList.remove("btn-error");
    }, 2000);
}

window.addEventListener('DOMContentLoaded', (event) => {
    useDefaultSSE = false;

    initSSE(parseSSE);

    let btn = document.getElementById("card-payment-pay");
    btn.addEventListener("click", (event) => {
        if (btn.classList.contains("loading")) {
            btn.classList.remove("loading");
        } else {
            btn.classList.add("loading");

            let amount = Math.round(parseFloat(document.getElementById("card-payment-amount").value) * 100);

            fetch("/request-payment-token", {
                method: "POST",
                headers: {
                    'Accept': 'application/json',
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify({
                    amount: amount,
                })
            }).catch((reason) => {
                btn.classList.add("btn-error");
        
                setTimeout(() => {
                    btn.classList.remove("btn-error");
                }, 2000);
            });
        }
    });
});
