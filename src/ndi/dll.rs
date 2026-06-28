//! Windows-only: locate the NDI runtime DLL at run time.
//!
//! `Processing.NDI.Lib.x64.dll` is delay-loaded (see build.rs), so the loader
//! does not resolve it until the first NDI call. Before that call we add the
//! NDI runtime directory to the DLL search path. The directory is taken from
//! the `NDI_RUNTIME_DIR_Vx` environment variables the NDI runtime installer
//! sets, newest version first.

/// Pick the runtime directory from candidate env-var values, in priority order.
/// Returns the first present, non-empty value.
fn pick_runtime_dir(candidates: &[Option<String>]) -> Option<String> {
    candidates
        .iter()
        .flatten()
        .find(|v| !v.trim().is_empty())
        .cloned()
}

/// The `NDI_RUNTIME_DIR_Vx` environment variables, newest version first.
const RUNTIME_DIR_VARS: [&str; 5] = [
    "NDI_RUNTIME_DIR_V6",
    "NDI_RUNTIME_DIR_V5",
    "NDI_RUNTIME_DIR_V4",
    "NDI_RUNTIME_DIR_V3",
    "NDI_RUNTIME_DIR_V2",
];

/// Resolve the NDI runtime directory using `get` to look up env vars,
/// preferring the newest runtime version.
fn resolve_runtime_dir(get: impl Fn(&str) -> Option<String>) -> Option<String> {
    let candidates: Vec<Option<String>> = RUNTIME_DIR_VARS.iter().map(|k| get(k)).collect();
    pick_runtime_dir(&candidates)
}

#[link(name = "kernel32")]
extern "system" {
    fn SetDllDirectoryW(lpPathName: *const u16) -> i32;
}

/// Add the NDI runtime directory to the DLL search path so the delay-loaded
/// `Processing.NDI.Lib.x64.dll` (see build.rs) resolves at run time without
/// being copied next to the executable. No-op when no runtime dir is set, in
/// which case the loader falls back to the default search order (PATH, etc.).
///
/// Must be called before the first NDI function call. `Ndi::new` does this.
pub fn add_runtime_to_dll_path() {
    use std::os::windows::ffi::OsStrExt;

    let Some(dir) = resolve_runtime_dir(|k| std::env::var(k).ok()) else {
        return;
    };
    let wide: Vec<u16> = std::ffi::OsStr::new(&dir)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    // Safety: `wide` is a valid NUL-terminated UTF-16 string living past the call.
    unsafe {
        SetDllDirectoryW(wide.as_ptr());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_first_present_candidate() {
        let c = [
            None,
            Some(r"C:\Program Files\NDI\NDI 6 Runtime\v6".to_string()),
            Some(r"C:\old\v5".to_string()),
        ];
        assert_eq!(
            pick_runtime_dir(&c),
            Some(r"C:\Program Files\NDI\NDI 6 Runtime\v6".to_string())
        );
    }

    #[test]
    fn skips_empty_and_whitespace_values() {
        let c = [
            Some(String::new()),
            Some("   ".to_string()),
            Some(r"C:\old\v4".to_string()),
        ];
        assert_eq!(pick_runtime_dir(&c), Some(r"C:\old\v4".to_string()));
    }

    #[test]
    fn none_when_all_absent() {
        let c = [None, None];
        assert_eq!(pick_runtime_dir(&c), None);
    }

    #[test]
    fn resolve_prefers_v6_over_older() {
        let get = |k: &str| match k {
            "NDI_RUNTIME_DIR_V6" => Some(r"C:\v6".to_string()),
            "NDI_RUNTIME_DIR_V5" => Some(r"C:\v5".to_string()),
            _ => None,
        };
        assert_eq!(resolve_runtime_dir(get), Some(r"C:\v6".to_string()));
    }

    #[test]
    fn resolve_falls_back_to_older_version() {
        let get = |k: &str| match k {
            "NDI_RUNTIME_DIR_V4" => Some(r"C:\v4".to_string()),
            _ => None,
        };
        assert_eq!(resolve_runtime_dir(get), Some(r"C:\v4".to_string()));
    }

    #[test]
    fn resolve_none_when_unset() {
        assert_eq!(resolve_runtime_dir(|_| None), None);
    }
}
