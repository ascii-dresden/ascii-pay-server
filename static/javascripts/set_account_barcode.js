function parseSSE(data) {
    if (data && data.Qr && data.Qr.code) {

        let elements = document.getElementsByTagName("input");
        console.log(elements);

        toast("New barcode scanned: '" + data.Qr.code + "'", "Apply?", () => {
            for (let element of elements) {
                console.log(element);
                if (element.name && element.name.startsWith("barcode-")) {
                    element.value = data.Qr.code;
                }
            }
        });
    }
}

window.addEventListener('DOMContentLoaded', (event) => {
    initSSE(parseSSE)
});
