use leptos::html::Div;
use leptos::*;
use serde::{Deserialize, Serialize};
use std::rc::Rc;
use uuid::Uuid;
use wasm_bindgen::prelude::*;

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
            let chart_ref = chart_ref.get().expect("chart_ref is Some(), should not panic");
            request_animation_frame(move || {
                let chart_id = chart_ref.get_attribute("id").expect("chart div should always have id, but it didn't");
                let chart = EChart::init_by_id(&chart_id);
                set_chart(Some(chart.into()));
            });
        }
    });

    create_effect(cx, move |_| {
        chart.with(|chart| chart.as_ref().map(|chart| chart.set_option(options())));
    });

    view! {cx,
        <div id=Uuid::new_v4().to_string() node_ref=chart_ref style=style class=class></div>
    }
}

#[wasm_bindgen(module = "/src/echart.js")]
extern "C" {
    #[wasm_bindgen(js_name = "init_by_id")]
    fn init_by_id(id: JsValue) -> JsValue;
    #[wasm_bindgen(js_name = "set_options")]
    fn set_options(chart: &JsValue, options: JsValue);
}

struct EChart {
    chart: JsValue,
}

impl EChart {
    fn init_by_id(id: &str) -> EChart {
        EChart {
            chart: init_by_id(JsValue::from_str(id)),
        }
    }

    fn set_option(&self, options: ChartOptions) {
        set_options(&self.chart, serde_wasm_bindgen::to_value(&options).expect("chart options should be always serializable"))
    }
}

#[derive(Clone, Serialize, Deserialize)]
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

#[derive(Clone, Serialize, Deserialize)]
pub struct TitleOptions {
    pub text: String,
}

#[derive(Clone, Serialize, Deserialize)]
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
    Line { name: String, data: Vec<i32> },
    Bar { name: String, data: Vec<i32> },
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SeriesType {
    Line,
    Bar,
}

/// Acts as std::option::Option, but None is serialized into empty object instead of undefined
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(untagged)]
pub enum EmptyOption<T> {
    Some(T),
    None {},
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
