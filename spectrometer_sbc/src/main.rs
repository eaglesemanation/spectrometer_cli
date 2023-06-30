cfg_if::cfg_if! {
if #[cfg(feature = "ssr")] {
    use axum::{
        routing::{get, post},
        Router,
    };
    use tokio_stream::{wrappers::WatchStream, StreamExt};
    use futures::stream::{Stream};
    use axum::{response::{Sse, sse::{KeepAlive, Event}}, extract::State};
    use leptos::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use log::LevelFilter;
    use log4rs::append::console::ConsoleAppender;
    use log4rs::append::file::FileAppender;
    use log4rs::config::{Appender, Root};
    use spectrometer_sbc::app::*;
    use spectrometer_sbc::fallback::file_and_error_handler;
    use spectrometer_sbc::gpio::Pins;
    use std::convert::Infallible;
    use rppal::gpio::Level;
    use tokio::sync::watch::Receiver;
    use tokio::select;

    #[tokio::main]
    async fn main() -> anyhow::Result<()> {

        let stdout = ConsoleAppender::builder().build();
        let mut log_root = Root::builder().appender("stdout");
        let mut log_config =
        log4rs::Config::builder().appender(Appender::builder().build("stdout", Box::new(stdout)));

        match std::env::var("SPECTROMETER_SBC_LOG_PATH") {
            Ok(log_file_path) => {
                let file = FileAppender::builder().build(log_file_path)?;
                log_config = log_config.appender(Appender::builder().build("file", Box::new(file)));
                log_root = log_root.appender("file");
            }
            Err(std::env::VarError::NotPresent) => {}
            Err(err) => {
                panic!("{}", err);
            }
        }

        let _ = log4rs::init_config(log_config.build(log_root.build(LevelFilter::Info))?)?;

        log::info!(
            "Registered server functions: {:?}",
            leptos_server::server_fns_by_path()
        );

        let pins = Pins::init()?;

        // Setting get_configuration(None) means we'll be using cargo-leptos's env values
        // For deployment these variables are:
        // <https://github.com/leptos-rs/start-axum#executing-a-server-on-a-remote-machine-without-the-toolchain>
        // Alternately a file can be specified such as Some("Cargo.toml")
        // The file would need to be included with the executable when moved to deployment
        let conf = get_configuration(None).await?;
        let leptos_options = conf.leptos_options;
        let addr = leptos_options.site_addr;
        let routes = generate_route_list(|cx| view! { cx, <App/> }).await;

        // build our application with a route
        let app = Router::new()
            .route("/api/sse/trigger", get(trigger_state_handler).with_state(pins.trigger_state))
            .route("/api/*fn_name", post(leptos_axum::handle_server_fns))
            .leptos_routes(&leptos_options, routes, |cx| view! { cx, <App/> })
            .fallback(file_and_error_handler)
            .with_state(leptos_options);

        // run our app with hyper
        // `axum::Server` is a re-export of `hyper::Server`
        log::info!("listening on http://{}", &addr);
        let server = axum::Server::bind(&addr)
            .serve(app.into_make_service());
        let trigger_worker = pins.trigger_worker;

        select! {
            res = server => {res?},
            res = trigger_worker => {res??},
        }

        Ok(())
    }

    async fn trigger_state_handler(State(trigger_state): State<Receiver<Level>>) -> Sse<impl Stream<Item = Result<Event, Infallible>>>  {
        let stream = WatchStream::new(trigger_state).map(|state| {
            Ok(Event::default().json_data(state == Level::High).unwrap(/* safety: bool should always serialize */))
        });
        Sse::new(stream).keep_alive(KeepAlive::default())
    }
} else { // cfg(feature = "ssr")
    pub fn main() {
        // no client-side main function
        // unless we want this to work with e.g., Trunk for a purely client-side app
        // see lib.rs for hydration function instead
    }
}
}
