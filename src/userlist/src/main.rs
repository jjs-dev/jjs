mod client;
mod list_parse;

use graphql_client::GraphQLQuery;
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
    #[snafu(display("userlist is malformed: {}", description))]
    Format {
        description: String,
    },
    #[snafu(display("api error: {:?}", inner))]
    Frontend {
        inner: Vec<graphql_client::Error>,
    },
    #[snafu(display("transport error: {}", source))]
    Transport {
        source: frontend_api::TransportError,
    },
}

impl From<frontend_api::TransportError> for Error {
    fn from(source: frontend_api::TransportError) -> Self {
        Self::Transport { source }
    }
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
        let file = std::fs::read(&arg.file).context(ReadFile { filename: arg.file })?;
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

    let client = frontend_api::Client::from_env();

    for (login, password, groups) in data {
        let vars = client::create_user::Variables {
            login,
            password,
            groups,
        };

        let req_body = client::CreateUser::build_query(vars);
        let resp = client.query::<_, client::create_user::ResponseData>(&req_body)?;

        let res = resp.into_result();

        if let Err(errs) = res {
            if ignore_failures {
                eprintln!("warning: user creation failed");
                for err in errs {
                    eprintln!("\t{}", err);
                }
            } else {
                return Err(Error::Frontend { inner: errs });
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
