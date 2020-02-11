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
        time_data.push({
            x: moment(start),
            y: line.transaction.before_credit / 100
        });
    }

    for (line of transaction_data) {
        time_data.push({
            x: moment(line.transaction.date),
            y: line.transaction.after_credit / 100
        });
    }

    if (transaction_data.length > 0) {
        let line = transaction_data[transaction_data.length - 1];
        time_data.push({
            x: moment(end),
            y: line.transaction.after_credit / 100
        });
    }

    var data = {
        datasets: [
            {
                label: "Overview",
                lineTension: 0,
                steppedLine: true,
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
                        ticks: {
                            min: transaction_start,
                            max: transaction_end
                        }
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
