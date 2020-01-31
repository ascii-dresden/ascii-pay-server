function parseSSE(data) {
    if (data && data.type === "qr-code") {
        let elements = document.getElementsByTagName("input");

        if (data.content.code.startsWith("https://pay.ascii.coffee?code=")) {
            let account_number = data.content.code
                .replace("https://pay.ascii.coffee?code=", "")
                .replace(/-/g," ");

            toast("New account number scanned: '" + account_number + "'", "Apply?", () => {
                for (let element of elements) {
                    if (element.name && element.name.startsWith("account_number")) {
                        element.value = account_number;
                    }
                }
            });
        } else {
            toast("New barcode scanned: '" + data.content.code + "'", "Apply?", () => {
                for (let element of elements) {
                    if (element.name && element.name.startsWith("barcode-")) {
                        element.value = data.content.code;
                    }
                }
            });
        }
    }

    if (data && data.type === "nfc-card") {
        let elements = document.getElementsByTagName("input");

        var id = data.content.id;
        if (data.content.writeable) {
            id = "ascii: " + id;
        }

        toast("New nfc card scanned: '" + data.content.name + "'<br/>(" + id + ")", "Apply?", () => {
            for (let element of elements) {
                if (element.name && element.name.startsWith("nfc-new")) {
                    element.value = id;
                }
            }
        });
    }
}

window.addEventListener('DOMContentLoaded', (event) => {
    initSSE(parseSSE)
});
