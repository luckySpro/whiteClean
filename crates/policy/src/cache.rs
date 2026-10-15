use std::path::Path;

const DENY_COMPONENTS: &[&str] = &[
    "huggingface",
    "jetbrains",
    "com.apple.fontregistry",
    "com.apple.spotlight",
    "cookies",
    "cookies.binarycookies",
    "login data",
    "login data-journal",
    "history",
    "history-journal",
    "web data",
    "bookmarks",
    "local storage",
    "session storage",
    "sessions",
    "keychains",
    "preferences",
];

const CHROMIUM_VENDORS: &[(&str, &str)] = &[
    ("Google", "Chrome"),
    ("BraveSoftware", "Brave-Browser"),
    ("Microsoft Edge", "Edge"),
    ("Chromium", "Chromium"),
];

const CACHE_DIR_NAMES: &[&str] = &["Cache", "Code Cache", "GPUCache"];

pub fn is_regenerable_cache_path(path: &Path, home: &Path) -> bool {
    if denied(path) {
        return false;
    }
    let caches = home.join("Library").join("Caches");
    let logs = home.join("Library").join("Logs");
    let inside = |root: &Path| path.starts_with(root) && path != root;
    inside(&caches) || inside(&logs) || is_browser_profile_cache(path, home)
}

pub fn is_browser_profile_cache(path: &Path, home: &Path) -> bool {
    if denied(path) {
        return false;
    }
    let support = home.join("Library").join("Application Support");
    let Ok(rest) = path.strip_prefix(&support) else {
        return false;
    };
    let parts: Vec<String> = rest
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    // The cache directory itself is not a file. Require a child path under it.
    if parts.len() < 5 {
        return false;
    }
    let chromium = CHROMIUM_VENDORS
        .iter()
        .any(|(vendor, product)| parts[0] == *vendor && parts[1] == *product);
    if chromium && CACHE_DIR_NAMES.contains(&parts[3].as_str()) {
        return true;
    }
    parts[0] == "Firefox" && parts[1] == "Profiles" && parts[3] == "cache2"
}

pub fn cache_component_denied(name: &str) -> bool {
    let name = name.to_lowercase();
    DENY_COMPONENTS.iter().any(|item| name == *item)
}

fn denied(path: &Path) -> bool {
    path.components().any(|component| {
        cache_component_denied(&component.as_os_str().to_string_lossy())
    })
}
