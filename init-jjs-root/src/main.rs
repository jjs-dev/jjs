use structopt::StructOpt;

fn main() {
    if let Err(e) = init_jjs_root::init_jjs_root(init_jjs_root::Args::from_args()) {
        eprintln!("Error: {}", e.source);
        eprintln!("At: {:?}", e.backtrace);
    }
}
