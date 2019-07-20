mod list_parse;

use snafu::{ResultExt, Snafu};
use std::process::exit;
use structopt::StructOpt;

use list_parse::StatementData;

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
    Network {
        source: frontend_api::NetworkError,
    },
}

fn decode_base64(s: String) -> String {
    let buf = base64::decode(&s).unwrap_or_else(fail);
    String::from_utf8(buf).unwrap_or_else(fail)
}

#[derive(Clone)]
struct OptionStorage {
    base64: bool,
    groups: Vec<String>,
    ignore_fail: bool,
}

impl OptionStorage {
    fn new() -> OptionStorage {
        OptionStorage {
            base64: false,
            groups: Vec::new(),
            ignore_fail: false,
        }
    }

    fn flag(&mut self, flag: &str) {
        match flag {
            "base64" => {
                self.base64 = true;
            }
            "ignore-fail" => {
                self.ignore_fail = true;
            }
            _ => {
                eprintln!("unknown flag: {}", flag);
                exit(1);
            }
        }
    }

    fn add_groups(&mut self, spec: &str) {
        let items = spec.split(':');
        for item in items {
            self.groups.push(item.to_string());
        }
    }

    fn setting(&mut self, name: &str, value: &str) {
        match name {
            "groups" => {
                self.add_groups(value);
            }
            "set-groups" => {
                self.groups.clear();
                self.add_groups(value);
            }
            _ => {
                eprintln!("unknown setting: {}", name);
                exit(1);
            }
        }
    }

    fn options(&mut self, opts: &[list_parse::Option]) {
        for option in opts {
            match option {
                list_parse::Option::Flag(flag) => {
                    self.flag(flag);
                }
                list_parse::Option::Setting(name, val) => {
                    self.setting(name, val);
                }
            }
        }
    }
}

fn add_users(arg: args::Add) -> Result<(), Error> {
    let mut data = Vec::new();
    let ignore_failures;
    {
        let file = std::fs::read(&arg.file).context(ReadFile {
            filename: arg.file.clone(),
        })?;
        let file = String::from_utf8(file).context(Utf8)?;

        let statements = list_parse::parse(&file);
        let mut option_storage = OptionStorage::new();
        for st in statements {
            match st.data {
                StatementData::SetOpt { options } => option_storage.options(&options),
                StatementData::AddUser {
                    mut username,
                    mut password,
                    options,
                } => {
                    let mut subst = option_storage.clone();
                    subst.options(&options);
                    let mut groups = subst.groups.clone();
                    if subst.base64 {
                        username = decode_base64(username);
                        password = decode_base64(password);
                        for item in &mut groups {
                            let dec = decode_base64(std::mem::replace(item, String::new()));
                            std::mem::replace(item, dec);
                        }
                    }

                    data.push((username, password, groups));
                }
            }
        }
        ignore_failures = option_storage.ignore_fail;
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

        let user_create_res = client.users_create(&req);
        let mut err = None;
        match user_create_res {
            Ok(Ok(_)) => {}
            Ok(Err(fr_err)) => {
                err = Some(Error::Frontend {
                    inner: Box::new(fr_err),
                });
            }
            Err(network_error) => {
                err = Some(Error::Network {
                    source: network_error,
                });
            }
        }

        if let Some(err) = err {
            if ignore_failures {
                eprintln!("note: user creation error: {}", err);
            } else {
                return Err(err);
            }
        }
    }

    Ok(())
}

fn fail<E: std::fmt::Display, X>(err: E) -> X {
    eprintln!("Error:\n{}", err);
    std::process::exit(1)
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
