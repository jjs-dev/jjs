use std::{
    process::{Stdio, Command},
    env::var
};

fn main() {
    let out_path = format!("{}/proto.rs", var("OUT_DIR").unwrap());
    let thrift = Command::new("thrift")
        .arg("-gen")
        .arg("rs")
        .arg("./proto.thrift")
        .arg("-out")
        .arg(&out_path);
}