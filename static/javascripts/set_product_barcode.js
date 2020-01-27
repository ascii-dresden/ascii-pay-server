function parseSSE(data) {
    if (data && data.type === "qr-code") {

        let elements = document.getElementsByClassName("barcode-target");

        toast("New barcode scanned: '" + data.content.code + "'", "Apply?", () => {
            for (let element of elements) {
                element.value = data.content.code;
            }
        });
    }
}

window.addEventListener('DOMContentLoaded', (event) => {
    initSSE(parseSSE)
});
