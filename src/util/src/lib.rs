pub mod cfg;
pub mod cmd;
pub mod log;

pub fn print_error(err: &dyn std::error::Error) {
    eprintln!("error: {}", err);
    let mut iter = err.source();
    while let Some(cause) = iter {
        eprintln!("caused by: {}", cause);
        iter = cause.source();
    }
}
