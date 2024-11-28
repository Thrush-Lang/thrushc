#[inline]
pub fn extract_file_name(path: &str) -> String {
    path.split('/').next().unwrap().to_string()
}
