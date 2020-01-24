function parseSSE(data) {
    if (data && data.Qr && data.Qr.code) {

        let elements = document.getElementsByClassName("barcode-target");

        toast("New barcode scanned: '" + data.Qr.code + "'", "Apply?", () => {
            for (let element of elements) {
                element.value = data.Qr.code;
            }
        });
    }
}

window.addEventListener('DOMContentLoaded', (event) => {
    initSSE(parseSSE)
});
