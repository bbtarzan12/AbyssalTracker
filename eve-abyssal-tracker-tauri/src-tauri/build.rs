fn main() {
    // package.json에서 버전 읽기
    let package_json = std::fs::read_to_string("../package.json")
        .expect("Failed to read package.json");
    
    let package: serde_json::Value = serde_json::from_str(&package_json)
        .expect("Failed to parse package.json");
    
    let version = package["version"].as_str()
        .expect("Version not found in package.json");
    
    // Cargo에 버전 정보 전달
    println!("cargo:rustc-env=CARGO_PKG_VERSION={}", version);
    
    tauri_build::build()
}
