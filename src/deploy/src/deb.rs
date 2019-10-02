//! Generates Debian packages (.deb)

use crate::{util::ensure_exists, Params};

use crate::util::make_empty;
use std::{
    fs,
    io::{BufWriter, Write},
    path::Path,
};

fn create_control_ar(params: &Params, workdir: &Path) {
    ensure_exists(workdir).unwrap();
    let control_dir_path = workdir.join("files");
    ensure_exists(&control_dir_path).unwrap();
    let source = params.src.join("deb/control.txt");
    let source = std::fs::read_to_string(source).unwrap();
    let mut dest = std::fs::File::create(control_dir_path.join("control")).unwrap();
    for line in source.lines() {
        dest.write_all(line.as_bytes()).unwrap();
        dest.write_all(b"\n").unwrap();
    }

    // now generate archive
    let dest_file_path = workdir.join("out.tar");
    let dest_file = fs::File::create(dest_file_path).unwrap();
    let dest_file = BufWriter::new(dest_file);
    let mut builder = tar::Builder::new(dest_file);
    let items = fs::read_dir(&control_dir_path).unwrap();
    for item in items {
        let item = item.unwrap();
        let item_path = item.path();
        let ty = item.file_type().unwrap();
        if ty.is_dir() {
            builder.append_dir_all(item.file_name(), item_path).unwrap();
        } else {
            builder
                .append_file(item.file_name(), &mut fs::File::open(item_path).unwrap())
                .unwrap();
        }
    }
    builder.finish().unwrap();
}

fn create_version_file(workdir: &Path) {
    let p = workdir.join("debian-binary");
    fs::write(p, "2.0").unwrap();
}

pub fn create(params: &Params) {
    let workdir = params.build.join("deb");
    ensure_exists(&workdir).unwrap();
    make_empty(&workdir).unwrap();
    println!("workdir: {}", workdir.display());
    let out_dir = workdir.join("out");
    fs::create_dir(&out_dir).unwrap();
    create_control_ar(params, &workdir.join("control"));
    fs::copy(workdir.join("control/out.tar"), out_dir.join("control.tar")).unwrap();
    create_version_file(&out_dir);
}
