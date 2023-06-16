export function init_by_id(id) {
    return echarts.init(document.getElementById(id));
}

export function set_options(chart, options) {
    console.log(options)
    chart.setOption(options);
}
