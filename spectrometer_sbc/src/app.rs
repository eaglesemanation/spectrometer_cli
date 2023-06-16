use crate::{echart::*, error_template::ErrorTemplate};
use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use log::{info};

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
            <main class="my-0 mx-auto max-w-3xl text-center">
                <Routes>
                    <Route path="" view=|cx| view! { cx, <HomePage/> }/>
                </Routes>
            </main>
        </Router>
    }
}

#[server(ListSerialPorts, "/api")]
pub async fn list_serial_ports() -> Result<Vec<String>, ServerFnError> {
    serialport::available_ports()
        .map(|ports| {
            ports
                .into_iter()
                .map(|port| port.port_name)
                .collect::<Vec<_>>()
        })
        .map_err(|err| ServerFnError::ServerError(err.to_string()))
}

#[server(ToggleLaser, "/api")]
async fn toggle_laser(cx: Scope) -> Result<(), ServerFnError> {
    use crate::gpio;

    let pins = gpio::get_pins().map_err(|err| ServerFnError::ServerError(err.to_string()))?;
    let mut led_pin = pins
        .laser_pin
        .lock()
        .map_err(|err| ServerFnError::ServerError(err.to_string()))?;
    info!("Toggling LED");
    led_pin.toggle();

    Ok(())
}

#[component]
fn HomePage(cx: Scope) -> impl IntoView {
    let ports = create_resource(cx, || {}, |_| async move { list_serial_ports().await });
    let ports_view = move || {
        ports
            .read(cx)
            .map(|ports| ports.map(|ports| {
                if ports.is_empty() {
                    view! {cx, <p>"No ports found"</p>}.into_view(cx)
                } else {
                    view! {cx,
                        <select class="block rounded-lg p-2.5">
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

    let serv_toggle_laser = create_server_action::<ToggleLaser>(cx);

    view! { cx,
        <Transition fallback=move || view!{cx, <p>"Loading.."</p>} >
            <ErrorBoundary fallback=move |cx, errors| view!{cx, <ErrorTemplate errors=errors/>}>
                {ports_view}
            </ErrorBoundary>
        </Transition>
        <button
            on:click=move |_| { serv_toggle_laser.dispatch(ToggleLaser{}); }
            class="bg-amber-600 disabled:bg-gray-400 hover:bg-sky-700 px-5 py-3 text-white rounded-lg"
            disabled=serv_toggle_laser.pending()
        >
            "Toggle LED"
        </button>
    }
}
