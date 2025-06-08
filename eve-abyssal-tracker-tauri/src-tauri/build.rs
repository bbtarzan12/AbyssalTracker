fn main() {
    // tauri.conf.json에서 버전 읽기
    let tauri_conf_json = std::fs::read_to_string("tauri.conf.json")
        .expect("Failed to read tauri.conf.json");
    
    let tauri_conf: serde_json::Value = serde_json::from_str(&tauri_conf_json)
        .expect("Failed to parse tauri.conf.json");
    
    let version = tauri_conf["version"].as_str()
        .expect("Version not found in tauri.conf.json");
    
    // Cargo에 버전 정보 전달
    println!("cargo:rustc-env=CARGO_PKG_VERSION={}", version);
    
    tauri_build::build()
}
