function init_main_diagram() {
    let container = document.getElementById("main-diagram");
    let canvas = document.createElement("canvas");
    container.appendChild(canvas);
    let ctx = canvas.getContext('2d');

    var time_data = [];
    var value = 0;

    for (line of transaction_data) {
        let t = line.total / 100;
        value += t;

        let d = moment(line.date)
        time_data.push({
            x: d,
            y: value
        });
    }

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
                    label: function(tooltipItem, chart, x){
                        var datasetLabel = chart.datasets[tooltipItem.datasetIndex];
                        console.log(tooltipItem, chart, datasetLabel, x);
                        return parseFloat(tooltipItem.value).toFixed(2) + "€";
                    }
                }
            },
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
