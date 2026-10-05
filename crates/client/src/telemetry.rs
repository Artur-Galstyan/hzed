use clock::SystemClock;
use gpui::App;
use http_client::HttpClientWithUrl;
use regex::Regex;
use std::sync::{Arc, LazyLock};
use worktree::{UpdatedEntriesSet, WorktreeId};

pub struct Telemetry;

pub fn os_name() -> String {
    #[cfg(target_os = "macos")]
    {
        "macOS".to_string()
    }
    #[cfg(target_os = "linux")]
    {
        format!("Linux {}", gpui::guess_compositor())
    }
    #[cfg(target_os = "freebsd")]
    {
        format!("FreeBSD {}", gpui::guess_compositor())
    }

    #[cfg(target_os = "windows")]
    {
        "Windows".to_string()
    }
}

/// Note: This might do blocking IO! Only call from background threads
pub fn os_version() -> String {
    cfg_select! {
       feature = "test-support" => {
           // MacOS branch in particular is quite slow, hence we ought to "avoid" it in tests.
           "test binary".to_owned()
       }
       target_os = "macos" => {
           static MACOS_VERSION_REGEX: LazyLock<Regex> = LazyLock::new(|| {
               Regex::new(r"(\s*\(Build [^)]*[0-9]\))").unwrap()
           });
           use objc2_foundation::NSProcessInfo;
           let process_info = NSProcessInfo::processInfo();
           let version_nsstring = process_info.operatingSystemVersionString();
           // "Version 15.6.1 (Build 24G90)" -> "15.6.1 (Build 24G90)"
           let version_string = version_nsstring.to_string().replace("Version ", "");
           // "15.6.1 (Build 24G90)" -> "15.6.1"
           // "26.0.0 (Build 25A5349a)" -> unchanged (Beta or Rapid Security Response; ends with letter)
           MACOS_VERSION_REGEX
               .replace_all(&version_string, "")
               .to_string()
       }
       any(target_os = "linux", target_os = "freebsd") => {
           use std::path::Path;

           let content = if let Ok(file) = std::fs::read_to_string(&Path::new("/etc/os-release")) {
               file
           } else if let Ok(file) = std::fs::read_to_string(&Path::new("/usr/lib/os-release")) {
               file
           } else if let Ok(file) = std::fs::read_to_string(&Path::new("/var/run/os-release")) {
               file
           } else {
               log::error!(
                   "Failed to load /etc/os-release, /usr/lib/os-release, or /var/run/os-release"
               );
               "".to_string()
           };
           util::parse_os_release(&content).unwrap_or_else(|| "unknown".to_string())
       }
       target_os = "windows" => {
           let mut info = unsafe { std::mem::zeroed() };
           let status = unsafe { windows::Wdk::System::SystemServices::RtlGetVersion(&mut info) };
           if status.is_ok() {
               semver::Version::new(
                   info.dwMajorVersion as _,
                   info.dwMinorVersion as _,
                   info.dwBuildNumber as _,
               )
               .to_string()
           } else {
               "unknown".to_string()
           }
       }
    }
}

impl Telemetry {
    pub fn new(
        _clock: Arc<dyn SystemClock>,
        _client: Arc<HttpClientWithUrl>,
        _cx: &mut App,
    ) -> Arc<Self> {
        Arc::new(Self)
    }

    pub fn metrics_enabled(self: &Arc<Self>) -> bool {
        false
    }

    pub fn set_authenticated_user_info(
        self: &Arc<Self>,
        _metrics_id: Option<String>,
        _is_staff: bool,
    ) {
    }

    pub fn log_edit_event(self: &Arc<Self>, _environment: &'static str, _is_via_ssh: bool) {}

    pub fn report_discovered_project_type_events(
        self: &Arc<Self>,
        _worktree_id: WorktreeId,
        _updated_entries_set: &UpdatedEntriesSet,
    ) {
    }

    pub fn metrics_id(self: &Arc<Self>) -> Option<Arc<str>> {
        None
    }

    pub fn system_id(self: &Arc<Self>) -> Option<Arc<str>> {
        None
    }

    pub fn is_staff(self: &Arc<Self>) -> Option<bool> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn telemetry_has_no_state_or_user_ids() {
        let telemetry = Arc::new(Telemetry);
        telemetry.set_authenticated_user_info(Some("user".into()), true);
        telemetry.log_edit_event("editor", false);

        assert_eq!(std::mem::size_of::<Telemetry>(), 0);
        assert!(telemetry.system_id().is_none());
        assert!(telemetry.metrics_id().is_none());
        assert!(telemetry.is_staff().is_none());
        assert!(!telemetry.metrics_enabled());
    }
}
