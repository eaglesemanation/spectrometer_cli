use leptos::html::Div;
use leptos::*;
use std::rc::Rc;
use uuid::Uuid;
use wasm_bindgen::prelude::*;
use web_sys::HtmlDivElement;
use web_sys::HtmlElement;
use serde::{Deserialize, Serialize};

#[wasm_bindgen]
extern "C" {
    #[derive(Debug, Clone)]
    type EChart;

    #[wasm_bindgen(js_namespace = echarts, js_name = "init")]
    fn init_echart(el: &HtmlElement) -> EChart;
    #[wasm_bindgen(method, js_name = "setOption")]
    fn set_option(this: &EChart, options: JsValue);
}

#[component]
pub fn Chart<F>(
    cx: Scope,
    options: F,
    #[prop(optional)] style: &'static str,
    #[prop(optional)] class: &'static str,
) -> impl IntoView
where
    F: Fn() -> ChartOptions + 'static,
{
    let (chart, set_chart) = create_signal::<Option<Rc<EChart>>>(cx, None);
    let chart_ref = create_node_ref::<Div>(cx);

    create_effect(cx, move |_| {
        if chart.with(|chart| chart.is_none()) && chart_ref.get().is_some() {
            let node = chart_ref
                .get()
                .expect("chart_ref is Some(), should not panic");
            let html_node = node.unchecked_ref::<HtmlDivElement>();
            if html_node.id().is_empty() {
                let id = Uuid::new_v4().to_string();
                log::info!("Changing id to {id}");
                node.clone().id(id);
            }
            node.on_mount(move |node| {
                log::info!("Initializing chart");
                let chart = init_echart(&node);
                set_chart(Some(chart.into()));
            });
        }
    });

    create_effect(cx, move |_| {
        log::info!("Updating chart");
        chart.with(|chart| {
            chart
                .as_ref()
                .map(|chart| chart.set_option(serde_wasm_bindgen::to_value(&options()).unwrap()))
        });
    });

    view! {cx,
        <div node_ref=chart_ref style=style class=class></div>
    }
}

#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChartOptions {
    pub title: TitleOptions,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tooltip: Option<TooltipOptions>,
    pub data_zoom: Vec<DataZoom>,
    pub x_axis: EmptyOption<AxisOptions>,
    pub y_axis: EmptyOption<AxisOptions>,
    pub series: Vec<Series>,
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct TitleOptions {
    pub text: String,
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct TooltipOptions {
    pub trigger: TooltipTrigger,
}

#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum TooltipTrigger {
    Item,
    Axis,
    #[default]
    None,
}

#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum DataZoom {
    #[default]
    Slider,
    Inside,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AxisOptions {
    pub data: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Series {
    Line { name: String, data: Vec<f64> },
    Bar { name: String, data: Vec<f64> },
}

/// Acts as std::option::Option, but None is serialized into empty object instead of undefined
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(untagged)]
pub enum EmptyOption<T> {
    Some(T),
    None {},
}

impl<T> Default for EmptyOption<T> {
    fn default() -> Self {
        EmptyOption::None {}
    }
}

impl<T> From<EmptyOption<T>> for Option<T> {
    fn from(empty_option: EmptyOption<T>) -> Option<T> {
        match empty_option {
            EmptyOption::Some(option) => Some(option),
            EmptyOption::None {} => None,
        }
    }
}

impl<T> From<Option<T>> for EmptyOption<T> {
    fn from(option: Option<T>) -> EmptyOption<T> {
        match option {
            Some(option) => EmptyOption::Some(option),
            None {} => EmptyOption::None {},
        }
    }
}

impl<T> EmptyOption<T> {
    pub fn into_option(self) -> Option<T> {
        self.into()
    }
    pub fn as_option(&self) -> Option<&T> {
        match self {
            EmptyOption::Some(option) => Some(option),
            EmptyOption::None {} => None,
        }
    }
}
