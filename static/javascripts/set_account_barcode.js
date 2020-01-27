function parseSSE(data) {
    if (data && data.type === "qr-code") {
        let elements = document.getElementsByTagName("input");

        toast("New barcode scanned: '" + data.content.code + "'", "Apply?", () => {
            for (let element of elements) {
                if (element.name && element.name.startsWith("barcode-")) {
                    element.value = data.content.code;
                }
            }
        });
    }

    if (data && data.type === "nfc-card") {
        let elements = document.getElementsByTagName("input");

        var id = data.content.id;
        if (data.content.writeable) {
            id = "ascii: " + id;
        }

        toast("New nfc card scanned: '" + id + "'", "Apply?", () => {
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
