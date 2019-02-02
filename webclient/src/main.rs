#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate failure;

mod config;
mod render;
mod session;
//mod medium;

use self::session::Session;
use frontend_api::TJjsServiceSyncClient;
use multipart::server::{save::SavedData, Multipart, SaveResult};
use rocket::{fairing::AdHoc, http::ContentType, request::Form, response::Redirect, Data, State};
use rocket_contrib::templates::Template;
use std::{
    io::{self, Read},
    sync::Arc,
};
use thrift::{
    protocol::{TCompactInputProtocol, TCompactOutputProtocol},
    transport::{
        ReadHalf, TFramedReadTransport, TFramedWriteTransport, TIoChannel, TTcpChannel, WriteHalf,
    },
};

#[derive(PartialEq)]
enum Environment {
    Development,
    Production,
}

struct AppEnvState {
    env: Environment,
}

//const INTERNAL_ERR_MSG: &str = "Internal Error";

#[derive(Fail, Debug)]
pub enum HttpError {
    //#[fail(display = "Io error: {}", _0)]
    //Io(#[cause] reqwest::Error),
    //#[fail(display = "Serialization error: {}", _0)]
    //Serde(#[cause] serde_json::Error),
    #[fail(display = "Internal Error")]
    Io(#[cause] io::Error),
    #[fail(display = "Error in interacting with JJS frontend")]
    Thrift(#[cause] thrift::Error),
    #[fail(display = "Incorrect request: {}", _0)]
    BadRequest(String),
}

impl From<thrift::Error> for HttpError {
    fn from(e: thrift::Error) -> HttpError {
        HttpError::Thrift(e)
    }
}

impl From<io::Error> for HttpError {
    fn from(e: io::Error) -> HttpError {
        HttpError::Io(e)
    }
}

#[get("/")]
fn route_index(ses: Session) -> Template {
    let mut ctx = render::DefaultRenderContext::default();
    ctx.common.fill_with_session_data(&ses.data);
    ctx.set_debug_info();
    Template::render("index", &ctx)
}

#[get("/login")]
fn route_login_page() -> Template {
    Template::render("login", &render::DefaultRenderContext::default())
}

#[get("/favicon.ico")]
fn route_favicon() -> std::io::Result<rocket::response::NamedFile> {
    rocket::response::NamedFile::open("./favicon.ico".to_string())
}

#[derive(FromForm)]
struct LoginForm {
    login: String,
    #[allow(dead_code)]
    password: String,
}

#[post("/authenticate", data = "<form>")]
fn route_authentificate(
    mut ses: session::Session,
    form: Form<LoginForm>,
    //mut provider_state: State<JjsClientProvider>,
    env_state: State<AppEnvState>,
) -> Result<Redirect, HttpError> {
    let form_data = form.into_inner();
    let auth_query = frontend_api::SimpleAuthParams {
        login: form_data.login.clone(),
        password: form_data.password,
    };

    let mut api_client = connect("127.0.0.1:1779".to_string()).unwrap();

    let auth_resp = api_client.simple(auth_query)?;

    ses.clear();
    ses.data.auth = Some(session::Auth {
        username: form_data.login,
        api_token: base64::encode(&auth_resp.buf),
    });

    if env_state.env == Environment::Development {
        ses.expose();
    }

    Ok(Redirect::to("/"))
}

#[get("/logout")]
fn route_logout(mut session: Session) -> Redirect {
    session.clear();
    Redirect::to("/")
}

#[get("/submit")]
fn route_get_submit(session: Session) -> Result<Template, Redirect> {
    let mut ctx = render::DefaultRenderContext::default();
    ctx.common.fill_with_session_data(&session.data);
    if let Some(ref _auth) = session.data.auth {
        Ok(Template::render("submit", &ctx))
    } else {
        Err(Redirect::to("/login"))
    }
}

#[post("/submit", data = "<data>")]
fn route_post_submit(
    cont_type: &ContentType,
    data: Data,
    //cfg: State<config::Config>,
) -> Result<Redirect, HttpError> {
    use std::io::Write;
    if !cont_type.is_form_data() {
        return Err(HttpError::BadRequest(
            "Content-Type is not multipart/form-data".into(),
        ));
    }

    let ctype = cont_type.params().find(|&(k, _)| k == "boundary");

    let boundary = match ctype {
        Some((_a, b)) => b,
        None => {
            return Err(HttpError::BadRequest(
                "Content-Type doesn't contain boundary".into(),
            ));
        }
    };

    let mut form = Multipart::with_body(data.open(), boundary);
    let form = form.save().temp();
    let form = match form {
        SaveResult::Full(entries) => entries,
        SaveResult::Partial(_partial, reason) => {
            let mut out = Vec::new();
            writeln!(out, "Handling failed: {:?}", reason).unwrap();
            return Err(HttpError::BadRequest(
                String::from_utf8_lossy(&out).to_string(),
            ));
        }
        SaveResult::Error(e) => return Err(HttpError::BadRequest(e.to_string())),
    };
    let toolchain_field_name: Arc<str> = Arc::from("toolchain");
    let toolchain: &Vec<multipart::server::save::SavedField> = &form.fields[&toolchain_field_name];
    let toolchain = &toolchain[0].data;
    let toolchain = match toolchain {
        SavedData::Bytes(_) | SavedData::File(_, _) => {
            return Err(HttpError::BadRequest("toolchain field must be text".into()));
        }
        SavedData::Text(ref s) => s,
    };
    println!("toolchain: {}", toolchain);
    //now we register submission
    let file = &form.fields[&Arc::from("code")][0].data;
    let mut file = file.readable().unwrap();
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)?;
    connect("127.0.0.1:1779".to_string())?.submit(frontend_api::SubmitDeclaration {
        toolchain: toolchain.clone(),
        code: contents,
    })?;
    Ok(Redirect::to("/"))
}

type ThriftInputProtocol = TCompactInputProtocol<TFramedReadTransport<ReadHalf<TTcpChannel>>>;
type ThriftOutputProtocol = TCompactOutputProtocol<TFramedWriteTransport<WriteHalf<TTcpChannel>>>;

fn connect(
    api_addr: String,
) -> thrift::Result<frontend_api::JjsServiceSyncClient<ThriftInputProtocol, ThriftOutputProtocol>> {
    println!("connecting to {}", &api_addr);
    let mut chan = thrift::transport::TTcpChannel::new();
    chan.open(&api_addr).unwrap();

    let (i_chan, o_chan) = chan.split()?;

    let i_tran = TFramedReadTransport::new(i_chan);
    let o_tran = TFramedWriteTransport::new(o_chan);

    let i_prot = TCompactInputProtocol::new(i_tran);
    let o_prot = TCompactOutputProtocol::new(o_tran);

    Ok(frontend_api::JjsServiceSyncClient::new(i_prot, o_prot))
}

fn main() {
    println!("starting JJS HttpClient");

    let handlers = routes![
        route_index,
        route_login_page,
        route_authentificate,
        route_get_submit,
        route_post_submit,
        route_logout,
        route_favicon,
    ];

    let config = config::Config {
        //sysroot: jjs_sysroot_path,
    };

    rocket::ignite()
        .mount("/", handlers)
        .attach(Template::fairing())
        .attach(AdHoc::on_attach("Configure", |rocket| {
            let env_name = rocket.config().get_str("env").unwrap_or("prod");
            let mut st = AppEnvState {
                env: Environment::Production,
            };
            if env_name == "dev" {
                st.env = Environment::Development;
            }
            Ok(rocket.manage(st))
        }))
        .manage(config)
        .launch();
}
