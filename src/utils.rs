use std::borrow::{Cow};
use std::path::{Path, PathBuf};

// Forcibly converts separators in a filepath to unix separators to
// to ensure that ZipArchive's by_name method will retrieve the proper
// file. Failing to convert to unix-style on Windows causes the
// ZipArchive not to find the file.
pub fn convert_path_separators(root_base: impl AsRef<Path>, href: &str) -> PathBuf {
    let path = root_base
        .as_ref()
        .join(href.split('/').collect::<PathBuf>());

    if cfg!(windows) {
        let path = path.as_path().display().to_string().replace('\\', "/");
        PathBuf::from(path)
    } else {
        path
    }
}

/// Decode the provided input if it contains percent encoded values (e.g, URLs).
pub fn percent_decode(input: &str) -> Option<Cow<str>> {
    percent_encoding::percent_decode(input.as_bytes())
        .decode_utf8()
        .ok()
}
