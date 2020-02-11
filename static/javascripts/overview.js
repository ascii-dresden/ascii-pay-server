function init_main_diagram() {
    let container = document.getElementById("main-diagram");
    let canvas = document.createElement("canvas");
    container.appendChild(canvas);
    let ctx = canvas.getContext('2d');

    transaction_data.reverse();
    var time_data = [];

    for (line of transaction_data) {
        time_data.push({
            x: moment(line.transaction.date),
            y: line.transaction.before_credit / 100
        });
        time_data.push({
            x: moment(line.transaction.date),
            y: line.transaction.after_credit / 100
        });
    }

    /*
    if (transaction_data.length > 0) {
        let line = transaction_data[transaction_data.length - 1];
        if (line.before_credit === 0) {
            time_data.push({
                x: moment(line.date),
                y: 0
            });
        }
    }
    */

    var data = {
        datasets: [
            {
                label: "Scatter Dataset",
                lineTension: 0,
                borderColor: "rgba(41, 128, 185,1.0)",
                backgroundColor: "rgba(41, 128, 185,0.2)",
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
                        type: "time",

                    }
                ],
                yAxes: [
                    {
                        beginAtZero: true,
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
                    label: function (tooltipItem, chart, x) {
                        var datasetLabel = chart.datasets[tooltipItem.datasetIndex];
                        return parseFloat(tooltipItem.value).toFixed(2) + "€";
                    }
                }
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
