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

use self::session::Session;
use multipart::server::{save::SavedData, Multipart, SaveResult};
use rocket::{
    fairing::AdHoc,
    http::ContentType,
    request::{Form, Request},
    response::{Redirect, Responder, Response},
    Data, State,
};
use rocket_contrib::templates::Template;
use std::{
    io::{self, Read},
    sync::Arc,
};

#[derive(PartialEq)]
enum Environment {
    Development,
    Production,
}

struct AppEnvState {
    env: Environment,
}

#[derive(Fail, Debug)]
pub enum HttpError {
    #[fail(display = "Internal Serialization error")]
    Serde(#[cause] serde_json::Error),
    #[fail(display = "Internal Error")]
    Io(#[cause] io::Error),
    #[fail(display = "Error in interacting with JJS frontend")]
    Frontend(#[cause] reqwest::Error),
    #[fail(display = "Incorrect request: {}", _0)]
    BadRequest(String),
}

impl<'r> Responder<'r> for HttpError {
    fn respond_to(self, _request: &Request) -> Result<Response<'r>, rocket::http::Status> {
        eprintln!("warning: responding with error {:?}", &self);
        let st = match self {
            HttpError::Serde(_) | HttpError::Io(_) | HttpError::Frontend(_) => {
                rocket::http::Status::InternalServerError
            }
            HttpError::BadRequest(_msg) => rocket::http::Status::BadRequest,
        };

        Err(st)
    }
}

impl From<io::Error> for HttpError {
    fn from(e: io::Error) -> HttpError {
        HttpError::Io(e)
    }
}

impl From<reqwest::Error> for HttpError {
    fn from(e: reqwest::Error) -> HttpError {
        HttpError::Frontend(e)
    }
}

#[get("/")]
fn route_index(ses: Session) -> Template {
    let mut ctx = render::DefaultRenderContext::default();
    ctx.common.fill_with_session_data(&ses.data);
    ctx.set_debug_info();
    eprintln!("{}", serde_json::to_string(&ctx).unwrap());
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
    env_state: State<AppEnvState>,
) -> Result<Redirect, HttpError> {
    let form_data = form.into_inner();
    let auth_query = frontend_api::SimpleAuthParams {
        login: form_data.login.clone(),
        password: form_data.password,
    };

    let client = reqwest::Client::new();
    let auth_resp: frontend_api::AuthToken = client
        .post("http://localhost:1779/auth/simple")
        .body(serde_json::to_string(&auth_query).unwrap())
        .send()?
        .json()?;

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
fn route_post_submit(cont_type: &ContentType, data: Data) -> Result<Redirect, HttpError> {
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
    let contents = base64::encode(&contents);

    let frontend_query = frontend_api::SubmitDeclaration {
        toolchain: 0, //TODO
        code: contents,
    };
    let _id: frontend_api::SubmissionId = reqwest::Client::new()
        .post("http://localhost:1779/submissions/send")
        .body(serde_json::to_string(&frontend_query).unwrap())
        .send()?
        .json()
        .unwrap();

    Ok(Redirect::to("/"))
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

    let config = config::Config {};

    rocket::ignite()
        .mount("/", handlers)
        .attach(Template::custom(|e| {
            e.handlebars.set_strict_mode(true);
        }))
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
