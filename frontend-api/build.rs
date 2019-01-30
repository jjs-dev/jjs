use std::{env::var, process::Command};

fn main() {
    let thrift = var("THRIFTC").unwrap_or("/opt/thrift/bin/thrift".to_string());
    let exit_code = Command::new(thrift)
        .arg("-gen")
        .arg("rs")
        .arg("-out")
        .arg("./src")
        .arg("./proto.thrift")
        .status()
        .unwrap();

    assert_eq!(exit_code.success(), true);
    println!("cargo:rerun-if-changed=proto.thrift");
}
