mod proxy;

use clap::{App, Arg};
use warp::Filter;

fn cli() {
    const ARG_CONFIG: &str = "config";
    const ARG_DEBUG: &str = "debug";

    let app = App::new("Proxy Front")
        .version(env!("FULL_VERSION"))
        .about("Proxy")
        .arg(
            Arg::with_name(ARG_CONFIG)
                .long("config")
                .value_name("config.yaml")
                .help("Set the configuration file to use")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(ARG_DEBUG)
                .long("debug")
                .help("Set loglevel to debug (override config file level)"),
        );

    let matches = app.get_matches();

    let override_debug = matches.is_present(ARG_DEBUG);

    let config_file = matches.value_of(ARG_CONFIG).unwrap_or("config.yaml");
    ()
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    cli();

    let www = warp::fs::dir("www");

    let bind = ([0, 0, 0, 0], 3030);
    warp::serve(warp::path("api").and(proxy::api()).or(www))
        .run(bind)
        .await
}
