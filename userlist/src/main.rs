use snafu::{ResultExt, Snafu};
use std::process::exit;
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
        /// JJS frontend host or IP
        #[structopt(long = "host", short = "h", default_value = "http://localhost")]
        pub host: String,
        /// JJS frontend port
        #[structopt(long = "port", short = "p", default_value = "1779")]
        pub port: u16,
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
    #[snafu(display("userlist is malformed: {}", & description))]
    Format {
        description: String,
    },
    #[snafu(display("frontend returned error: {:?}", & inner))]
    Frontend {
        inner: Box<dyn frontend_api::FrontendError>,
    },
}

enum ParseLineOutcome {
    Comment,
    User(String, String, Vec<String>),
    Error(String),
}

fn decode_base64(s: &str) -> Option<String> {
    let bytes = base64::decode(s).ok()?;
    let s = String::from_utf8(bytes).ok()?;
    Some(s)
}

fn decode_value_list(s: &str, cfg: &Config) -> Option<Vec<String>> {
    let elems = s.split(':');
    let mut out = Vec::new();
    for elem in elems {
        match decode_value(elem, cfg) {
            Some(s) => out.push(s),
            None => return None,
        };
    }
    Some(out)
}

struct Config {
    base64: bool,
}

fn decode_value(s: &str, cfg: &Config) -> Option<String> {
    if cfg.base64 {
        decode_base64(s)
    } else {
        Some(s.to_string())
    }
}

fn parse_header(line: &str) -> Config {
    if !line.starts_with('!') {
        eprintln!("error: header doesn't start from !");
        exit(1);
    }
    let items: Vec<_> = line
        .splitn(2, ' ')
        .nth(1)
        .unwrap_or("")
        .split(',')
        .collect();
    let base64 = items.contains(&"BASE64");
    Config { base64 }
}

fn parse_line(line: &str, cfg: &Config) -> ParseLineOutcome {
    if line.starts_with('#') {
        return ParseLineOutcome::Comment;
    }
    let parts: Vec<_> = line.split_whitespace().collect();
    if parts.len() != 3 {
        return ParseLineOutcome::Error(format!(
            "Line must contain 3 whitespace-separated items, but got {}",
            parts.len()
        ));
    }
    let username = match decode_value(&parts[0], cfg) {
        Some(s) => s,
        None => return ParseLineOutcome::Error("Username has invalid format".to_string()),
    };
    let password = match decode_value(&parts[1], cfg) {
        Some(s) => s,
        None => return ParseLineOutcome::Error("Password has invalid format".to_string()),
    };
    let groups = match decode_value_list(&parts[2], cfg) {
        Some(s) => s,
        None => return ParseLineOutcome::Error("Groups has invalid format".to_string()),
    };
    ParseLineOutcome::User(username, password, groups)
}

fn add_users(arg: args::Add) -> Result<(), Error> {
    let mut data = Vec::new();
    {
        let file = std::fs::read(&arg.file).context(ReadFile {
            filename: arg.file.clone(),
        })?;
        let file = String::from_utf8(file).context(Utf8)?;
        let mut lines = file.lines();
        let header_line = match lines.next() {
            Some(s) => s,
            None => {
                return Err(Error::Format {
                    description: "Header entry mising (file is empty)".to_string(),
                });
            }
        };
        let cfg = parse_header(header_line);
        for (i, line) in lines.enumerate() {
            let outcome = parse_line(line, &cfg);
            let entry = match outcome {
                ParseLineOutcome::Error(desc) => {
                    let description = format!("line {}: {}", i, desc);
                    return Err(Error::Format { description });
                }
                ParseLineOutcome::Comment => continue,
                ParseLineOutcome::User(us, pw, grs) => (us, pw, grs),
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

    let endpoint = format!("{}:{}", &arg.host, &arg.port);

    let client = frontend_api::Client {
        endpoint,
        token,
        logger: None,
    };
    for (login, password, groups) in data {
        let req = frontend_api::UsersCreateParams {
            login,
            password,
            groups,
        };
        match client.users_create(&req).expect("network error") {
            Ok(_) => {}
            Err(e) => return Err(Error::Frontend { inner: Box::new(e) }),
        }
    }

    Ok(())
}

fn main() {
    let arg: args::Args = args::Args::from_args();
    let args::Args::Add(arg) = arg;
    let res = add_users(arg);
    match res {
        Ok(_) => (),
        Err(e) => {
            eprintln!("error: {}", e);
            exit(1);
        }
    }
}
