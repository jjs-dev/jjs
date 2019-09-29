//! This module implements support for systemd config generation
//! Templates are in systemd/*, we just want to compile them and put to appropriate place
use serde::Serialize;
use tera::compile_templates;

#[derive(Serialize)]
struct TplCtxt {
    jjs_sysroot: String,
}

pub(crate) fn build(params: &crate::Params) {
    if let Some(install_prefix) = &params.install_prefix {
        let tpls = compile_templates!("systemd/**/*");
        let tplcx = TplCtxt {
            jjs_sysroot: install_prefix.display().to_string(),
        };
        let emit_unit = |unit_name| {
            let unit_tpl_name = format!("jjs-{}.service.tera", unit_name);
            let unit_text = tpls
                .render(&unit_tpl_name, &tplcx)
                .expect("unit interpolation failed");
            let out_path = params
                .artifacts
                .join(format!("lib/systemd/system/jjs-{}.service", unit_name));
            std::fs::write(out_path, unit_text).expect("failed emit unit");
        };
        emit_unit("backend");
        emit_unit("frontend");
    } else {
        eprintln!("error: systemd files can not be generated without --install-prefix");
        std::process::exit(1);
    }
}
