use clap::Parser;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, value_parser)]
    message: Option<String>,
}

fn main() {
    let args = Args::parse();

    match args.message {
        Some(msg) => println!("{}", msg),
        None => println!("Hello world!"),
    }
}
