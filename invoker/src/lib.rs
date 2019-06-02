mod simple_invoker;

use cfg::Config;

#[derive(Debug)]
pub struct SubmissionInfo {
    /// Ancestor for all other directories in this struct
    root_dir: String,
    /// Directory to share with sandbox
    share_dir: String,
    /// Directory which will be chroot for sandbox
    chroot_dir: String,
    /// Submission toolchain name
    toolchain: String,
}

impl SubmissionInfo {
    pub fn new(sysroot: &str, submission_id: u32, invokation_id: &str, toolchain: &str) -> Self {
        let submission_root_dir = format!("{}/var/submissions/s-{}", sysroot, submission_id);
        let submission_chroot_dir =
            format!("{}/chroot-build-{}", &submission_root_dir, invokation_id);
        let submission_share_dir =
            format!("{}/share-build-{}", &submission_root_dir, invokation_id);
        SubmissionInfo {
            chroot_dir: submission_chroot_dir,
            root_dir: submission_root_dir,
            share_dir: submission_share_dir,
            toolchain: toolchain.to_string(),
        }
    }
}

pub fn invoke(
    submission_info: SubmissionInfo,
    logger: &slog::Logger,
    cfg: &Config,
) -> invoker_api::Status {
    dbg!(&submission_info);
    simple_invoker::judge(&submission_info, cfg, logger)
}
