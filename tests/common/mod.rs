use archflow::compression::CompressionMethod;

pub mod std;
pub mod tokio;

pub const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");

#[allow(dead_code)]
pub fn out_file_name(compressor: CompressionMethod, test_id: &str) -> String {
    ["test_", &compressor.to_string(), "_", test_id, ".zip"].join("")
}
