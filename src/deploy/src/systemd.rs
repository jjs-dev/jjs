//! This module implements support for systemd config generation
//! Templates are in systemd/*, we just want to compile them and put to appropriate place
use serde::Serialize;

#[derive(Serialize)]
struct TplCtx {
    jjs_sysroot: String,
}

pub(crate) fn build(params: &crate::Params) {
    if let Some(install_prefix) = &params.install_prefix {
        let tpls = tera::Tera::new("systemd/**/*").expect("failed to compile tera templates");
        let mut tpl_render_ctx = tera::Context::new();
        tpl_render_ctx.insert("jjs_sysroot", &install_prefix.display().to_string());
        let emit_unit = |unit_name| {
            let unit_tpl_name = format!("jjs-{}.service.tera", unit_name);
            let unit_text = tpls
                .render(&unit_tpl_name, &tpl_render_ctx)
                .expect("unit interpolation failed");
            let out_path = params
                .artifacts
                .join(format!("lib/systemd/system/jjs-{}.service", unit_name));
            std::fs::write(out_path, unit_text).expect("failed emit unit");
        };
        emit_unit("invoker");
        emit_unit("apiserver");
    } else {
        eprintln!("error: systemd files can not be generated without --install-prefix");
        std::process::exit(1);
    }
}
