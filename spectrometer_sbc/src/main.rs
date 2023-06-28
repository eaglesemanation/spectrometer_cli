#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::{routing::post, Router};
    use leptos::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use log::LevelFilter;
    use log4rs::append::console::ConsoleAppender;
    use log4rs::append::file::FileAppender;
    use log4rs::config::{Appender, Root};
    use spectrometer_sbc::app::*;
    use spectrometer_sbc::fallback::file_and_error_handler;

    let stdout = ConsoleAppender::builder().build();
    let mut log_root = Root::builder().appender("stdout");
    let mut log_config =
        log4rs::Config::builder().appender(Appender::builder().build("stdout", Box::new(stdout)));

    match std::env::var("SPECTROMETER_SBC_LOG_PATH") {
        Ok(log_file_path) => {
            let file = FileAppender::builder().build(log_file_path).unwrap();
            log_config = log_config.appender(Appender::builder().build("file", Box::new(file)));
            log_root = log_root.appender("file");
        }
        Err(std::env::VarError::NotPresent) => {}
        Err(err) => {
            panic!("{}", err);
        }
    }

    let _ =
        log4rs::init_config(log_config.build(log_root.build(LevelFilter::Info)).unwrap()).unwrap();

    log::info!("Registered server functions: {:?}", leptos_server::server_fns_by_path());

    // Setting get_configuration(None) means we'll be using cargo-leptos's env values
    // For deployment these variables are:
    // <https://github.com/leptos-rs/start-axum#executing-a-server-on-a-remote-machine-without-the-toolchain>
    // Alternately a file can be specified such as Some("Cargo.toml")
    // The file would need to be included with the executable when moved to deployment
    let conf = get_configuration(None).await.unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(|cx| view! { cx, <App/> }).await;

    // build our application with a route
    let app = Router::new()
        .route("/api/*fn_name", post(leptos_axum::handle_server_fns))
        .leptos_routes(&leptos_options, routes, |cx| view! { cx, <App/> })
        .fallback(file_and_error_handler)
        .with_state(leptos_options);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    log!("listening on http://{}", &addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for a purely client-side app
    // see lib.rs for hydration function instead
}
