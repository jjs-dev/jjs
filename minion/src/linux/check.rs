use libc::*;

/// Validate environment
///
/// If some problems are present, Some(s) with returned, where s is human-readable string
///describing these problems
pub fn check() -> Option<String> {
    let uid = unsafe { getuid() };
    if uid != 0 {
        return Some(format!("Running with uid={} instead of 0", uid));
    }

    let cap_info = match procfs::Process::myself()
        .and_then(|p| p.status())
        .map(|st| st.capeff)
    {
        Ok(p) => p,
        Err(e) => return Some(format!("couldn't get capabilities: {}", e)),
    };

    const REQUIRED_CAPS: u64 = 7 /*CAP_SETUID*/;
    if cap_info & REQUIRED_CAPS != REQUIRED_CAPS {
        return Some("some required capabilities are missing".to_string());
    }

    None
}
