use snafu::{ResultExt, Snafu};
use structopt::StructOpt;

mod args {
    use structopt::StructOpt;

    #[derive(StructOpt)]
    pub struct Add {
        /// File to add users from. see `Userlist` man page
        pub file: String,
        /// Auth token. If not set, will be read from JJS_AUTH environment variable
        #[structopt(long = "auth", short = "a")]
        pub token: Option<String>,
    }

    #[derive(StructOpt)]
    pub enum Args {
        #[structopt(name = "add")]
        Add(Add),
    }
}

#[derive(Snafu, Debug)]
enum Error {
    ReadFile {
        filename: String,
        source: std::io::Error,
    },
    Utf8 {
        source: std::string::FromUtf8Error,
    },
    #[snafu(display("userlist is malformed: {}", &description))]
    Format {
        description: String,
    },
}

enum ParseLineOutcome {
    Comment,
    User(String, String),
    Error(String),
}

fn parse_line(line: &str) -> ParseLineOutcome {
    if line.starts_with('#') {
        return ParseLineOutcome::Comment;
    }
    let parts: Vec<_> = line.split_whitespace().collect();
    if parts.len() != 2 {
        return ParseLineOutcome::Error(format!(
            "Line must contain 2 items, but got {}",
            parts.len()
        ));
    }
    ParseLineOutcome::User(parts[0].into(), parts[1].into())
}

fn add_users(arg: args::Add) -> Result<(), Error> {
    let mut data = Vec::new();
    {
        let file = std::fs::read(&arg.file).context(ReadFile {
            filename: arg.file.clone(),
        })?;
        let file = String::from_utf8(file).context(Utf8)?;
        let lines = file.lines();
        for (i, line) in lines.enumerate() {
            let outcome = parse_line(line);
            let entry = match outcome {
                ParseLineOutcome::Error(desc) => {
                    let description = format!("line {}: {}", i, desc);
                    return Err(Error::Format { description });
                }
                ParseLineOutcome::Comment => continue,
                ParseLineOutcome::User(us, pw) => (us, pw),
            };
            data.push(entry);
        }
    }

    let token = match arg.token {
        Some(tok) => tok.clone(),
        None => std::env::var("JJS_AUTH").unwrap_or_else(|_e| {
            eprintln!("neither --auth nor JJS_AUTH are not set");
            std::process::exit(1);
        }),
    };

    Ok(())
}

fn main() {
    let arg: args::Args = args::Args::from_args();
    let arg: args::Add = match arg {
        args::Args::Add(x) => x,
    };
    let res = add_users(arg);
    match res {
        Ok(_) => (),
        Err(e) => {
            eprintln!("error: {}", e);
        }
    }
}
