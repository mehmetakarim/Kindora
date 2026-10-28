fn main() {
    // En az yetki: ön yüz yalnız bu iki uygulama komutunu çağırabilir; her biri
    // `capabilities/main.json` içinde açıkça izinlidir.
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&["get_agent_state", "refresh_agent"]),
    ))
    .expect("tauri-build failed");
}
