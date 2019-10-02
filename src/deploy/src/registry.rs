use crate::{build_ctx::BuildCtx, inst_ctx::InstallCtx, pkg::Package, sel_ctx::SelCtx};

pub(crate) struct Registry {
    pkgs: Vec<Box<dyn Package>>,
}

impl Registry {
    pub(crate) fn new() -> Self {
        Self { pkgs: Vec::new() }
    }

    pub(crate) fn add<P: Package + 'static>(&mut self, pkg: P) {
        self.pkgs.push(Box::new(pkg));
    }

    pub(crate) fn run_selection(&mut self, sctx: &SelCtx) {
        for p in &mut self.pkgs {
            p.check_selected(sctx);
        }
    }

    pub(crate) fn build(&mut self, bctx: &BuildCtx) {
        for p in &mut self.pkgs {
            if p.selected() {
                p.build(bctx);
            }
        }
    }

    pub(crate) fn install(&mut self, ictx: &InstallCtx) {
        for p in &mut self.pkgs {
            if p.selected() {
                p.install(ictx);
            }
        }
    }
}
