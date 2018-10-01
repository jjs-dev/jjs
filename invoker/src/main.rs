//#![feature(plugin)]
//#![plugin(rocket_codegen)]

extern crate execute;
extern crate config;
extern crate invoker_task;
//extern crate rocket;
extern crate futures;
extern crate serde;
//#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate amqp;

mod invoker;
mod object;
mod simple_invoker;

use amqp::{Basic, Session, Table};
use invoker_task::*;
//use std::fs;
use std::path::{PathBuf};

const AMQP_URL: &str = "amqp://localhost//";
const QUEUE_NAME: &str = "jjs_invoker";

fn handle_judge_task(task: InvokeRequest, cfg: &config::Config) {
    let file_path = PathBuf::from(format!("{}/var/jjs/submits/{}", cfg.sysroot.to_string_lossy(), task.submission_name));
    if !file_path.exists() {
        println!("file not exists");
        return;
    }

    let submission = object::Submission::from_file_path(&file_path, &task.toolchain_name);

    let status = simple_invoker::judge(submission, cfg);

    println!("Judging result: {:#?}", status);
}

fn handle_task(task: Request, cfg: &config::Config) {
    match task {
        Request::Print(req) => {
            println!("print: {}", req.data);
        }
        Request::Exit(_req) => {
            println!("exiting.");
            std::process::exit(0);
        }
        Request::Invoke(req) => {
            println!("judging {}", &req.submission_name);
            handle_judge_task(req, cfg);
        }
        Request::Noop(_req) => {}
    }
}

fn main() {
    let config = config::get_config();
    println!("{:#?}", config);
    let mut session = Session::open_url(AMQP_URL).unwrap();
    let mut channel = session.open_channel(1).unwrap();
    let queue_declare = channel.queue_declare(QUEUE_NAME,
                                              false, true, false,
                                              false, false,
                                              Table::new());
    queue_declare.unwrap();
    loop {
        std::thread::sleep(std::time::Duration::from_millis(200));
        for get_result in channel.basic_get(QUEUE_NAME, true) {
            let body = String::from_utf8(get_result.body.clone()).unwrap();
            println!("got query: {}", body);
            let request: invoker_task::Request = serde_json::from_str(&body).unwrap();
            handle_task(request, &config);
        }
    }
}
