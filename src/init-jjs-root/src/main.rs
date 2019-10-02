use structopt::StructOpt;

fn main() {
    if let Err(e) = init_jjs_root::init_jjs_root(init_jjs_root::Args::from_args()) {
        eprintln!("error: {}", e.source);
        eprintln!("at: {:?}", e.backtrace);
        std::process::exit(1);
    }
}
