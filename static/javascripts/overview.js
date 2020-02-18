class Color {
    constructor(value) {
        if (value.startsWith('--')) {
            let name = value;
            value = getComputedStyle(document.documentElement)
                .getPropertyValue(name).trim();
        }

        if (value.startsWith('#')) {
            if (value.length == 4) {
                this.red = parseInt(value.substr(1, 1), 16) * 16;
                this.green = parseInt(value.substr(2, 1), 16) * 16;
                this.blue = parseInt(value.substr(3, 1), 16) * 16;
            } else {
                this.red = parseInt(value.substr(1, 2), 16);
                this.green = parseInt(value.substr(3, 2), 16);
                this.blue = parseInt(value.substr(5, 2), 16);
            }
            this.alpha = 1.0
        } else {
            value = input.split("(")[1].split(")")[0].split(",");
            this.red = parseInt(value[0]);
            this.green = parseInt(value[1]);
            this.blue = parseInt(value[2]);
            if (value.length > 3) {
                this.alpha = parseFloat(value[3]);
            } else {
                this.alpha = 1.0
            }
        }
    }

    toString(alpha) {
        if (!alpha) {
            alpha = this.alpha;
        }

        if (this.alpha >= 1.0) {
            var red = this.red.toString(16);
            red = red.length == 1 ? "0" + red : red;
            var green = this.green.toString(16);
            green = green.length == 1 ? "0" + green : green;
            var blue = this.blue.toString(16);
            blue = blue.length == 1 ? "0" + blue : blue;
            return '#' + red + green + blue;
        } else {
            return 'rgba(' + [this.red, this.green, this.blue, this.alpha].join(', ') + ')';
        }
    }
}

function main_diagram_tooltip(tooltip) {
    console.log(tooltip);

    let container = document.getElementById("main-diagram");
    var tooltipContainer = document.getElementById("main-diagram-tooltip");
    if (!tooltipContainer) {
        tooltipContainer = document.createElement("div");
        tooltipContainer.id = "main-diagram-tooltip";
        tooltipContainer.classList.add("diagram-tooltip")
        container.appendChild(tooltipContainer);
    }

    if (tooltip.body) {
        let index = tooltip.body[0].lines[0];
        let line = transaction_data[index];

        let total = parseFloat(line.transaction.total / 100).toFixed(2) + "€";
        let before = parseFloat(line.transaction.before_credit / 100).toFixed(2) + "€";
        let after = parseFloat(line.transaction.after_credit / 100).toFixed(2) + "€"

        var products = "";
        for (let prod of line.products) {
            products += `<span class="chip">${prod.amount} × ${prod.product.name}</span>`;
        }

        tooltipContainer.innerHTML = `<h5>${line.transaction.date}</h5>
        <table>
        <tr>
            <td>Total</td>
            <td>${total}</td>
        </tr>
        <tr>
            <td>Balance</td>
            <td>${before} → ${after}</td>
        </tr>
        <tr>
            <td>Products</td>
            <td>${products}</td>
        </tr>
        </table>`;

        tooltipContainer.style.left = (container.offsetLeft + tooltip.caretX) + "px";
        tooltipContainer.style.top = (container.offsetTop + tooltip.caretY) + "px";

        if (tooltip.xAlign === "right") {
            tooltipContainer.classList.add("right");
        } else {
            tooltipContainer.classList.add("left");
        }

        tooltipContainer.classList.add("active");
    } else {
        tooltipContainer.classList.remove("active", "right", "left");
    }
}

function init_main_diagram() {
    let container = document.getElementById("main-diagram");
    let canvas = document.createElement("canvas");
    container.appendChild(canvas);
    let ctx = canvas.getContext('2d');

    transaction_data.reverse();
    var time_data = [];

    let start = new Date(transaction_start);
    start.setDate(start.getDate() - 1);
    let end = new Date(transaction_end);
    end.setDate(end.getDate() + 1);

    if (transaction_data.length > 0) {
        let line = transaction_data[0];
        let y = line.transaction.before_credit / 100;
        time_data.push({
            x: moment(start),
            y: y
        });
    }

    for (line of transaction_data) {
        let y = line.transaction.before_credit / 100;
        time_data.push({
            x: moment(line.transaction.date),
            y: y
        });
    }

    if (transaction_data.length > 0) {
        let line = transaction_data[transaction_data.length - 1];
        let y = line.transaction.before_credit / 100;
        time_data.push({
            x: moment(end),
            y: y
        });
    }

    let primaryColor = new Color('--primary-color');
    let gridColor = new Color('--border-color');
    let textColor = new Color('--gray-color-dark');

    Chart.defaults.global.defaultFontColor = textColor.toString(0.3);

    var data = {
        datasets: [
            {
                label: "Overview",
                lineTension: 0,
                steppedLine: true,
                borderColor: primaryColor.toString(),
                backgroundColor: primaryColor.toString(0.2),
                fill: false,
                data: time_data
            }
        ]
    };

    new Chart(ctx, {
        type: "line",
        data: data,
        options: {
            animation: false,
            legend: {
                display: false
            },
            scales: {
                xAxes: [
                    {
                        scaleLabel: {
                            display: true
                        },
                        gridLines: {
                          color: gridColor.toString(),
                          zeroLineColor: textColor.toString()
                        },
                        color: textColor.toString(),
                        type: "time",
                        ticks: {
                            min: transaction_start,
                            max: transaction_end
                        }
                    }
                ],
                yAxes: [
                    {
                        beginAtZero: true,
                        gridLines: {
                          color: gridColor.toString(),
                          zeroLineColor: textColor.toString()
                        },
                        color: textColor.toString(),
                        ticks: {
                            callback: function (value) {
                                return value.toFixed(2) + "€";
                            }
                        }
                    }
                ]
            },
            tooltips: {
                callbacks: {
                    label: function (tooltipItem) {
                        return tooltipItem.index - 1;
                    }
                },
                enabled: false,
                mode: 'index',
                position: 'nearest',
                custom: main_diagram_tooltip
            },
            maintainAspectRatio: false,
            layout: {
                padding: {
                    top: 30,
                }
            }
        }
    });

}

window.addEventListener('DOMContentLoaded', () => {
    init_main_diagram();
});
