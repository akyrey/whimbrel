#![allow(clippy::unwrap_used, clippy::expect_used, dead_code)]

use std::path::PathBuf;

use url::Url;
use wbl_openapi::{Api, Loader, OpenApiError, build, load_all};

pub struct Disk;

impl Loader for Disk {
    async fn load(&self, url: &Url) -> Result<String, OpenApiError> {
        let path = url
            .to_file_path()
            .map_err(|()| OpenApiError::Load(format!("not a file: {url}")))?;
        std::fs::read_to_string(path).map_err(|e| OpenApiError::Load(e.to_string()))
    }
}

pub fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

pub fn api(name: &str) -> Api {
    let path = fixture(name);
    let text = std::fs::read_to_string(&path).unwrap();
    let url = Url::from_file_path(&path).unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let loaded = runtime
        .block_on(load_all(url, &text, &Disk, false))
        .unwrap();
    assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    build(loaded.documents).unwrap()
}
