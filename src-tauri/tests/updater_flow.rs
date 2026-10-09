#![cfg(target_os = "macos")]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::Duration;
use tauri_plugin_updater::UpdaterExt;

const ARCHIVE: &[u8] = include_bytes!("fixtures/update-test.app.tar.gz");
const SIGNATURE: &str = include_str!("fixtures/update-test.app.tar.gz.sig");

fn serve(version: &str, signature: &str, package: Vec<u8>, requests: usize) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let manifest = serde_json::json!({
        "version": version, "platforms": {"darwin-aarch64": {
            "url": format!("{base}/update"), "signature": signature.trim()
        }}
    }).to_string();
    std::thread::spawn(move || {
        for connection in listener.incoming().take(requests) {
            let mut stream = connection.unwrap();
            stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            let mut request = [0; 4096];
            let n = stream.read(&mut request).unwrap();
            let data = if String::from_utf8_lossy(&request[..n]).starts_with("GET /update ") {
                &package[..]
            } else { manifest.as_bytes() };
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", data.len()).unwrap();
            stream.write_all(data).unwrap();
        }
    });
    base
}

fn test_app(endpoint: &str) -> tauri::App<tauri::test::MockRuntime> {
    let config: serde_json::Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    let mut context = tauri::test::mock_context(tauri::test::noop_assets());
    context.package_info_mut().version = "1.3.0".parse().unwrap();
    context.config_mut().plugins.0.insert("updater".into(), serde_json::json!({
        // Local HTTP is enabled ONLY in this mock; shipped configuration enforces HTTPS.
        "dangerousInsecureTransportProtocol": true,
        "pubkey": config["plugins"]["updater"]["pubkey"],
        "endpoints": [endpoint]
    }));
    tauri::test::mock_builder().plugin(tauri_plugin_updater::Builder::new().build()).build(context).unwrap()
}

#[tokio::test]
async fn signed_update_installs_only_to_staged_app_and_rejects_tampering() {
    let dir = tempfile::tempdir().unwrap();
    let staged = dir.path().join("米奇.app");
    let binary = staged.join("Contents/MacOS/mickey-companion");
    std::fs::create_dir_all(binary.parent().unwrap()).unwrap();
    std::fs::write(&binary, "old version").unwrap();
    let diary = dir.path().join("private-diary.json");
    std::fs::write(&diary, "preserve user records").unwrap();

    let endpoint = serve("1.2.4", SIGNATURE, ARCHIVE.to_vec(), 1);
    let app = test_app(&endpoint);
    assert!(app.updater_builder().executable_path(&binary).target("darwin-aarch64").no_proxy().build().unwrap().check().await.unwrap().is_none());
    assert_eq!(std::fs::read_to_string(&binary).unwrap(), "old version");

    let mut corrupted = ARCHIVE.to_vec();
    corrupted[20] ^= 1;
    let endpoint = serve("9.0.0", SIGNATURE, corrupted, 2);
    let app = test_app(&endpoint);
    let update = app.updater_builder().executable_path(&binary).target("darwin-aarch64").no_proxy().build().unwrap().check().await.unwrap().unwrap();
    assert!(update.download(|_, _| {}, || {}).await.is_err());
    assert_eq!(std::fs::read_to_string(&binary).unwrap(), "old version");

    let endpoint = serve("9.0.0", SIGNATURE, ARCHIVE.to_vec(), 2);
    let app = test_app(&endpoint);
    let update = app.updater_builder().executable_path(&binary).target("darwin-aarch64").no_proxy().build().unwrap().check().await.unwrap().unwrap();
    update.download_and_install(|_, _| {}, || {}).await.unwrap();
    assert_eq!(std::fs::read_to_string(&binary).unwrap(), "updated fixture version");
    assert_eq!(std::fs::read_to_string(diary).unwrap(), "preserve user records");
}
