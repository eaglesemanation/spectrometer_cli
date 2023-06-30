use std::io::{Read, Write};

use crate::{components::chart::*, error_template::ErrorTemplate};
use ccd_lcamv06::{IoAdapter, StdIoAdapter};
use leptos::{html::Input, *};
use leptos_meta::*;
use leptos_router::*;

struct IOIgnoreWrite<T: Read>(T);

impl<T: Read> Read for IOIgnoreWrite<T> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buf)
    }
}

impl<T: Read> Write for IOIgnoreWrite<T> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[component]
pub fn App(cx: Scope) -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context(cx);

    view! {
        cx,
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/spectrometer_sbc.css"/>
        // Deps for Chart component
        <Script src="https://cdn.jsdelivr.net/npm/echarts@5.4.2/dist/echarts.min.js"/>
        <Title text="CCD Spectrometer"/>
        <Router>
            <main class="my-0 mx-auto max-w-6xl text-center">
                <Body class="bg-slate-50 dark:bg-slate-800"/>
                <Routes>
                    <Route path="" view=|cx| view! { cx, <HomePage/> }/>
                </Routes>
            </main>
        </Router>
    }
}

#[server(ToggleLaser, "/api")]
async fn toggle_laser(cx: Scope) -> Result<(), ServerFnError> {
    /*
    use crate::gpio;

    let pins = gpio::get_pins().map_err(|err| ServerFnError::ServerError(err.to_string()))?;
    let mut led_pin = pins
        .laser_pin
        .lock()
        .map_err(|err| ServerFnError::ServerError(err.to_string()))?;
    info!("Toggling LED");
    led_pin.toggle();
    */

    Ok(())
}

#[component]
fn HomePage(cx: Scope) -> impl IntoView {
    let (chart_data, set_chart_data) = create_signal(cx, vec![]);

    let chart_view = move || {
        let chart_options = move || ChartOptions {
            title: TitleOptions {
                text: "Spectrogram".to_string(),
            },
            x_axis: Some(AxisOptions {
                data: (0..chart_data().len()).map(|x| x.to_string()).collect(),
            })
            .into(),
            data_zoom: vec![DataZoom::Slider, DataZoom::Inside],
            series: vec![Series::Line {
                name: "Intensity".to_string(),
                data: chart_data(),
            }],
            tooltip: Some(TooltipOptions {
                trigger: TooltipTrigger::Axis,
            }),
            ..Default::default()
        };
        view! { cx, <Chart options=chart_options class="aspect-[4/3]"/> }
    };

    view! { cx,
        <Gpio/>
        <SerialPortReader set_frame=set_chart_data/>
        <FileReader set_frame=set_chart_data/>
        <Transition fallback=move || {} >
            <ErrorBoundary fallback=move |cx, errors| view!{cx, <ErrorTemplate errors=errors/>}>
                {chart_view}
            </ErrorBoundary>
        </Transition>
    }
}

#[component]
fn Gpio(cx: Scope) -> impl IntoView {
    let serv_toggle_laser = create_server_action::<ToggleLaser>(cx);

    #[cfg(not(feature = "ssr"))]
    let trigger_state = {
        use tokio_stream::StreamExt;

        let mut source = gloo_net::eventsource::futures::EventSource::new("/api/sse/trigger")
            .expect("couldn't connect to SSE stream");
        let s = create_signal_from_stream(
            cx,
            source
                .subscribe("message")
                .unwrap()
                .map(|value| match value {
                    Ok(value) => value.1.data().as_string().expect("expected string value"),
                    Err(_) => "false".to_string(),
                }),
        );

        on_cleanup(cx, move || source.close());
        s
    };

    #[cfg(feature = "ssr")]
    let (trigger_state, _) = create_signal(cx, None::<String>);

    view! { cx,
        <div
            class=move || {
                format!("m-3 flex h-10 w-10 items-center justify-center rounded-full {}", match trigger_state() {
                    Some(val) if val == "true" => "bg-green-600",
                    _ => "bg-green-950"
                })
            }
        ></div>
        <button
            on:click=move |_| { serv_toggle_laser.dispatch(ToggleLaser{}); }
            class="bg-rose-600 disabled:bg-gray-400 hover:bg-rose-800 p-3 text-white rounded-lg"
            disabled=serv_toggle_laser.pending()
        >
            "Toggle LED"
        </button>
    }
}

#[server(ListSerialPorts, "/api")]
pub async fn list_serial_ports() -> Result<Vec<String>, ServerFnError> {
    let ports =
        serialport::available_ports().map_err(|err| ServerFnError::ServerError(err.to_string()))?;
    let port_names = ports.into_iter().map(|port| port.port_name).collect();
    Ok(port_names)
}

#[server(GetSingleReading, "/api")]
pub async fn get_single_reading(port: String) -> Result<Vec<f64>, ServerFnError> {
    let serial = serialport::new(port, Default::default())
        .open()
        .map_err(|err| ServerFnError::ServerError(err.to_string()))?;
    let mut ccd = StdIoAdapter::new(serial).open_ccd();
    let frame = ccd
        .get_frame()
        .map_err(|err| ServerFnError::ServerError(err.to_string()))?;
    Ok(frame.into_iter().map(f64::from).collect())
}

#[component]
fn SerialPortReader(cx: Scope, set_frame: WriteSignal<Vec<f64>>) -> impl IntoView {
    let ports = create_local_resource(cx, || {}, |_| async move { list_serial_ports().await });
    let (selected_port, set_selected_port) = create_signal(cx, None);
    let ports_view = move || {
        ports
            .read(cx)
            .map(|ports| ports.map(|ports| {
                if ports.is_empty() {
                    view! {cx, <p>"No ports found"</p>}.into_view(cx)
                } else {
                    set_selected_port(ports.first().cloned());
                    view! {cx,
                        <select
                            on:change=move |ev| {
                                set_selected_port(Some(event_target_value(&ev)));
                            }
                            class="rounded-lg p-3"
                        >
                            {
                                ports
                                    .into_iter()
                                    .map(move |port| view! {cx, <option value=port.clone()>{port}</option>})
                                    .collect_view(cx)
                            }
                        </select>
                    }
                    .into_view(cx)
                }
            }))
    };

    let serv_get_single_reading = create_server_action::<GetSingleReading>(cx);
    create_effect(cx, move |_| {
        serv_get_single_reading
            .value()
            .get()
            .map(|frame| frame.map(set_frame))
    });

    view! { cx,
        <div class="m-3">
            {ports_view}
            <button
                class="bg-amber-600 disabled:bg-gray-400 hover:bg-amber-800 p-3 m-3 text-white rounded-lg"
                disabled=serv_get_single_reading.pending()
                on:click=move |_| {
                    match selected_port() {
                        Some(port) => {
                            serv_get_single_reading.dispatch(GetSingleReading{port});
                        },
                        None => {}
                    }
                }
            >
                "Get a single frame"
            </button>
        </div>
    }
}

#[component]
fn FileReader(cx: Scope, set_frame: WriteSignal<Vec<f64>>) -> impl IntoView {
    let file_ref = create_node_ref::<Input>(cx);
    let file_parse = move |_| {
        if let Some(files) = file_ref.get().and_then(|f| f.files()) {
            let file = files.get(0).unwrap();
            let file_blob_promise = js_sys::Promise::resolve(&file.array_buffer());
            spawn_local(async move {
                let bytes = wasm_bindgen_futures::JsFuture::from(file_blob_promise)
                    .await
                    .unwrap();
                let byte_vec = js_sys::Uint8Array::new(&bytes).to_vec();
                let file_str = std::str::from_utf8(&byte_vec).unwrap();
                let parsed_hex: Vec<_> = file_str
                    .split(&[' ', '\n'][..])
                    .filter(|s| s.len() == 2)
                    .map(|hex| u8::from_str_radix(hex, 16).unwrap())
                    .collect();
                let hex_cursor = IOIgnoreWrite(parsed_hex.as_slice());
                let mut ccd = ccd_lcamv06::StdIoAdapter::new(hex_cursor).open_ccd();
                let frame = ccd.get_frame().unwrap();
                let frame_vec = frame.into_iter().map(|x| x.into()).collect();
                set_frame(frame_vec);
            });
        }
    };

    view! { cx,
        <div class="m-3">
            <input type="file" node_ref=file_ref class="bg-slate-200 hover:bg-slate-400 p-3 w-64 rounded-lg"/>
            <button
                class="bg-amber-600 disabled:bg-gray-400 hover:bg-amber-800 p-3 m-3 text-white rounded-lg"
                on:click=file_parse
            >
                "Read from file"
            </button>
        </div>
    }
}
