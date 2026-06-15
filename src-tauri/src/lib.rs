//! VRCaster 后端入口：声明模块并注册所有 Tauri 命令。
//! 各模块实现见对应文件，契约见 docs/contract.md。

pub mod types;

pub mod config;
pub mod encode;
pub mod ffmpeg;
pub mod media;
pub mod queue;
pub mod server;
pub mod subtitle;
pub mod upload;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(crate::encode::ProcRegistry::default())
        .invoke_handler(tauri::generate_handler![
            media::analyze_media,
            media::default_sample_spec,
            encode::encode_sample,
            encode::start_encode,
            encode::cancel_encode,
            encode::pause_encode,
            encode::resume_encode,
            upload::start_upload,
            upload::check_remote_space,
            queue::run_queue,
            config::list_profiles,
            config::save_profile,
            config::delete_profile,
            config::generate_access_key,
            config::get_settings,
            config::set_settings,
            config::export_config,
            config::import_config,
            server::deploy_nginx,
            ffmpeg::ensure_ffmpeg,
        ])
        .run(tauri::generate_context!())
        .expect("运行 Tauri 应用出错");
}
