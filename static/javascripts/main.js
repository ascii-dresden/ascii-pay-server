/**
 * Init change events for all money input fields.
 */
function initMoneyInputs() {
    for (let input of document.getElementsByClassName("money-input")) {
        initMoneyInput(input)
    }
}

/**
 * Init change events for an money input field.
 * 
 * @param {HTMLInputElement} input The input field to apply listeners
 */
function initMoneyInput(input) {
    if (input.readOnly) {
        // Ignore read only inputs
        return;
    }
    
    // Select content on focus
    input.addEventListener("focus", (event) => {
        input.select();
    });

    // Capture key presses and change input value
    input.addEventListener("keydown", (event) => {
        if (event.ctrlKey || event.altKey) {
            // Ignore keys with modifier
            return
        }

        let isNumber = event.key >= '0' && event.key <= '9'
        let isSign = event.key === '-' || event.key === '+';
        let isDot = event.key === '.' || event.key == ',';

        var checkStartZero = false;

        if ([33, 34, 38, 40].includes(event.keyCode)) {
            // ArrowUp/Down or PageUp/Down pressed. 
            // Change value in 0.10€ or 1.00€ steps.
            var value = parseFloat(input.value);

            if (isNaN(value)) {
                // Current value is not a number. Init it with 0.00€
                value = 0;
            } else {
                switch (event.keyCode) {
                    case 33:
                        // PageUp - Increase by 1.00€
                        value = (Math.floor(value) + 1);
                        break;
                    case 34:
                        // PageDown - Decrease by 1.00€
                        value = (Math.ceil(value) - 1);
                        break;
                    case 38: 
                        // ArrowUp - Increase by 0.10€
                        value = (Math.floor(value * 10) + 1) / 10;
                        break;
                    case 40: 
                        // ArrowDown - Decrease by 0.10€
                        value = (Math.ceil(value * 10) - 1) / 10;
                        break;
                }
            }

            input.value = value.toFixed(2);
            event.preventDefault();
        } else if (isNumber) {
            // Number pressed

            let length = input.value.length;
            let dotPosition = input.value.indexOf('.');

            let startPos = input.selectionStart;
            let endPos = input.selectionEnd;
            let caretPosition = startPos === endPos ? startPos : -1;

            if (startPos != endPos) {
                // Replace selection.
                input.value = input.value.slice(0, startPos) + event.key + input.value.slice(endPos);
                
                input.setSelectionRange(startPos + 1, startPos + 1);
                event.preventDefault();
            } else if (caretPosition === length && input.value.match(/-?[0-9]{2}/)) {
                // Caret is at the end of the input and has at least 3 (2 + 1) charecters.
                // Insert a dot before the last 2 charecters.
                let value = input.value.replace('.', '') + event.key;
                input.value = value.slice(0, -2) + '.' + value.slice(-2)

                input.setSelectionRange(input.value.length, input.value.length);
                event.preventDefault();
            } else if (caretPosition + 2 >= length && dotPosition >= 0) {
                // Caret is at the last two chars and the input contains a dot.
                // Replace the next char.
                input.value = input.value.slice(0, caretPosition) + event.key + input.value.slice(caretPosition + 1);

                input.setSelectionRange(caretPosition + 1, caretPosition + 1);
                event.preventDefault();
            } else {
                // Insert a char at the cartet position.
                input.value = input.value.slice(0, caretPosition) + event.key + input.value.slice(caretPosition);

                input.setSelectionRange(caretPosition + 1, caretPosition + 1);
                event.preventDefault();
            }

            checkStartZero = true;
        } else if (isSign) {
            // +/- pressed
            let startPos = input.selectionStart;
            let endPos = input.selectionEnd;

            if (event.key === '-') {
                // Toggle -
                if (input.value.charAt(0) === '-') {
                    // Remove -
                    input.value = input.value.slice(1);
                    input.setSelectionRange(Math.max(0, startPos - 1), endPos - 1);
                } else {
                    // Add -
                    input.value = '-' + input.value;
                    input.setSelectionRange(startPos + 1, Math.min(input.value.length, endPos + 1));
                }
            } else if (input.value.charAt(0) === '-') {
                // Remove -
                input.value = input.value.slice(1);
                input.setSelectionRange(Math.max(0, startPos - 1), endPos - 1);
            }

            event.preventDefault();
            checkStartZero = true;
        } else if (isDot) {
            // . pressed
            var valueMut = input.value;
            var caretPositionMut = input.selectionStart;
            var dotPositionMut = input.value.indexOf('.');

            // Remove multiple .s
            while (dotPositionMut >= 0) {
                if (caretPositionMut > dotPositionMut) {
                    caretPositionMut -= 1
                }

                valueMut = valueMut.slice(0, dotPositionMut) + valueMut.slice(dotPositionMut + 1)
                dotPositionMut = valueMut.indexOf('.');
            }

            // Add . at caret position
            valueMut = valueMut.slice(0, caretPositionMut) + '.' + valueMut.slice(caretPositionMut);

            // Add mission 0s after .
            while (valueMut.length < caretPositionMut + 3) {
                valueMut += '0'
            }

            input.value = valueMut;
            input.setSelectionRange(caretPositionMut + 1, caretPositionMut + 1);

            event.preventDefault();
            checkStartZero = true;
        } else if (event.key.length == 1) {
            // Ignore other keys
            event.preventDefault();
        }

        if (checkStartZero) {
            // Fix 0 count before the dot
            let startPos = input.selectionStart;
            let endPos = input.selectionEnd;
            let oldLength = input.value.length;

            input.value = input.value
                .replace(/^(-?)0*([0-9]+)\./, "$1$2.")
                .replace(/^(-?)\./, "$10.")
                .replace(/(\.[0-9]{2}).*/, "$1");

            let diff = input.value.length - oldLength;
            input.setSelectionRange(startPos + diff, endPos + diff);
        }
    });
}

let eventHandlers = [];

function initSSE(onmessage) {
    if (eventHandlers.length == 0) {
        eventHandlers.push(onmessage)

        const evtSource = new EventSource("/events");
        evtSource.onmessage = function(event) {
            try {
                let data = JSON.parse(event.data);
                for (let on of eventHandlers) {
                    on(data);
                }
            } catch {
                // Nothing to do
            }
        }
    } else {
        eventHandlers.push(onmessage)
    }
}

function toast(message, actionLabel, actionCallback) {
    let toasts = document.body.getElementsByClassName("body-toast");
    for (let t of toasts) {
        document.body.removeChild(t);
    }

    let container = document.createElement("div");
    container.classList.add("body-toast");

    let toast = document.createElement("div");
    toast.classList.add("container", "grid-lg", "toast", "toast-primary");
    toast.textContent = message;
    container.appendChild(toast);
    
    let close = document.createElement("span");
    close.classList.add("btn", "btn-clear", "float-right");
    close.addEventListener("click", () => {
        let toasts = document.body.getElementsByClassName("body-toast");
        for (let t of toasts) {
            document.body.removeChild(t);
        }
    });
    toast.appendChild(close);

    let action = document.createElement("span");
    action.classList.add("toast-action", "float-right");
    action.textContent = actionLabel;
    action.addEventListener("click", () => {
        actionCallback();

        let toasts = document.body.getElementsByClassName("body-toast");
        for (let t of toasts) {
            document.body.removeChild(t);
        }
    });
    toast.appendChild(action);

    document.body.appendChild(container);
    setTimeout(() => {
        let toasts = document.body.getElementsByClassName("body-toast");
        for (let t of toasts) {
            document.body.removeChild(t);
        }
    }, 10000);
}

function parseGlobalSSE(data) {
    console.log(data);
    if (data && data.type === "product") {
        let path = "/product/" + data.content.id;
        if (window.location.pathname !== path) {
            var p = ""
            if (data.content.current_price) {
                p = " ("+(data.content.current_price / 100).toFixed(2)+"€)"
            }
            toast("Found product: '" + data.content.name + "'" + p, "Edit?", () => {
                window.location = path;
            });
        }
    }
    if (data && data.type === "account") {
        let path = "/account/" + data.content.id;
        if (window.location.pathname !== path) {
            toast("Found account: '" + data.content.name + "'", "Edit?", () => {
                window.location = path;
            });
        }
    }
}

/**
 * Init function is called after dom is completly loaded.
 */
window.addEventListener('DOMContentLoaded', (event) => {
    initMoneyInputs();
    initSSE(parseGlobalSSE);
});
