use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::process::Command;

pub fn permission_missing(home: &Path) -> bool {
    ["Desktop", "Documents", "Downloads"].into_iter().any(|name| {
        match fs::read_dir(home.join(name)) {
            Err(error) => error.kind() == ErrorKind::PermissionDenied,
            Ok(_) => false,
        }
    })
}

pub fn open_permission_settings() -> bool {
    Command::new("/usr/bin/open")
        .arg("x-apple.systempreferences:com.apple.settings.PrivacySecurity.extension?Privacy_AllFiles")
        .spawn()
        .is_ok()
}
