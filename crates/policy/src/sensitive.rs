use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensitiveHit {
    Name,
    Extension,
    Location,
}

const NAME_HINTS: &[&str] = &[
    "身份证",
    "护照",
    "合同",
    "税单",
    "纳税",
    "密码",
    "密钥",
    "私钥",
    "password",
    "passwd",
    "wallet",
    "seed phrase",
];

const EXTENSIONS: &[&str] = &["pem", "kdbx", "p12", "pfx"];

const LOCATION_COMPONENTS: &[&str] = &[
    ".ssh",
    "keychains",
    "mail",
    "messages",
    "wallets",
    "1password",
    "bitwarden",
    "keepass",
    "lastpass",
];

pub fn is_sensitive(path: &Path) -> Option<SensitiveHit> {
    if path
        .components()
        .any(|c| c.as_os_str().to_string_lossy().to_lowercase().ends_with(".photoslibrary"))
    {
        return Some(SensitiveHit::Location);
    }
    for component in path.components() {
        let name = component.as_os_str().to_string_lossy().to_lowercase();
        if LOCATION_COMPONENTS.iter().any(|hint| name == *hint) {
            return Some(SensitiveHit::Location);
        }
    }
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if file_name == "id_rsa" || file_name == "id_ed25519" || file_name.starts_with("id_rsa.") {
        return Some(SensitiveHit::Name);
    }
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if EXTENSIONS.iter().any(|item| ext.eq_ignore_ascii_case(item)) {
            return Some(SensitiveHit::Extension);
        }
    }
    if NAME_HINTS.iter().any(|hint| file_name.contains(hint)) {
        return Some(SensitiveHit::Name);
    }
    None
}
