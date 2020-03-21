mod list_parse;

use client::Api as _;
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
        /// JJS apiserver host or IP
        #[structopt(long = "host", short = "h", default_value = "http://localhost")]
        pub host: String,
        /// JJS apiserver port
        #[structopt(long = "port", short = "p", default_value = "1779")]
        pub port: u16,
    }

    #[derive(StructOpt)]
    pub enum Args {
        #[structopt(name = "add")]
        Add(Add),
    }
}

#[derive(thiserror::Error, Debug)]
enum Error {
    #[error("failed to read {filename}: {source}")]
    ReadFile {
        filename: String,
        #[source]
        source: std::io::Error,
    },
    #[error("userlist is malformed: {}", source)]
    Format {
        #[from]
        source: list_parse::ParseError,
    },
    #[error("api error: {source}")]
    Api {
        #[from]
        source: client::Error,
    },
    #[error("invalid base64 string")]
    Base64,
}

fn decode_base64(s: String) -> Result<String, crate::Error> {
    let buf = base64::decode(&s).map_err(|_| crate::Error::Base64)?;
    String::from_utf8(buf).map_err(|_| crate::Error::Base64)
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

async fn add_users(arg: args::Add) -> Result<(), Error> {
    let mut data = Vec::new();
    let ignore_failures;
    {
        let file = std::fs::read_to_string(&arg.file).map_err(|source| crate::Error::ReadFile {
            filename: arg.file,
            source,
        })?;

        let statements = list_parse::parse(&file)?;
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
                        username = decode_base64(username)?;
                        password = decode_base64(password)?;
                        for item in &mut groups {
                            let dec = decode_base64(std::mem::replace(item, String::new()))?;
                            std::mem::replace(item, dec);
                        }
                    }

                    data.push((username, password, groups));
                }
            }
        }
        ignore_failures = option_storage.ignore_fail;
    }

    let client = client::connect();

    for (login, password, groups) in data {
        let params = client::models::UserCreateParams {
            login,
            password,
            groups: Some(groups),
        };
        let resp = client.create_user(params).await;

        if let Err(err) = resp {
            if ignore_failures {
                eprintln!("warning: user creation failed: {}", err);
            } else {
                return Err(Error::Api { source: err });
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    let arg: args::Args = args::Args::from_args();
    let args::Args::Add(arg) = arg;
    let res = add_users(arg).await;
    match res {
        Ok(_) => (),
        Err(e) => {
            eprintln!("error: {}", e);
            exit(1);
        }
    }
}
