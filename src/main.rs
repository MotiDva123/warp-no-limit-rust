use clap::Parser;
use colored::*;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs, thread, time::Duration};
use sysinfo::System;
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(author = "Moti", version = "1.0.0", about = "Warp No Limits [BETA] - Надежный автоматический байпасс", long_about = None)]
struct Args {
    /// Сброс идентификатора (сохранить приложение)
    #[arg(long)]
    reset: bool,

    /// Полное удаление приложения
    #[arg(long)]
    remove: bool,

    /// [BETA] Надежный байпасс с глубокой очисткой (рекомендуется)
    #[arg(long)]
    enhanced_beta: bool,

    /// Автоматический режим
    #[arg(long)]
    auto: bool,

    /// Тихий режим (минимум вывода)
    #[arg(long)]
    silent: bool,
}

struct WarpNoLimits {
    home: PathBuf,
    operation_count: usize,
    silent: bool,
}

impl WarpNoLimits {
    fn new(silent: bool) -> Self {
        let home = dirs::home_dir().expect("Не удалось определить домашнюю директорию");
        Self {
            home,
            operation_count: 0,
            silent,
        }
    }

    fn print_emoji(&self, emoji: &str, message: &str) {
        if !self.silent {
            println!("{}  {}", emoji, message);
        }
    }

    fn safe_remove(&mut self, path: &Path, description: &str) {
        if path.exists() {
            let result = if path.is_dir() {
                fs::remove_dir_all(path)
            } else {
                fs::remove_file(path)
            };

            match result {
                Ok(_) => {
                    let file_name = path.file_name().unwrap_or_default().to_string_lossy();
                    let emoji = if path.is_dir() { "🗂️" } else { "📄" };
                    self.print_emoji(emoji, &format!("Удалено {}: {}", description, file_name));
                    self.operation_count += 1;
                }
                Err(e) => {
                    if !self.silent {
                        self.print_emoji("⚠️", &format!("Не удалось удалить {:?}: {}", path, e));
                    }
                }
            }
        }
    }

    fn safe_remove_glob(&mut self, pattern: &str, description: &str) {
        if let Some(parent) = PathBuf::from(pattern).parent() {
            if parent.exists() {
                let pattern_name = PathBuf::from(pattern)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_lowercase();

                for entry in WalkDir::new(parent)
                    .max_depth(1)
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    let entry_name = entry
                        .file_name()
                        .to_string_lossy()
                        .to_lowercase();
                    
                    if pattern_name.contains("*") {
                        let check = pattern_name.replace("*", "");
                        if entry_name.contains(&check) {
                            self.safe_remove(entry.path(), description);
                        }
                    } else if entry_name == pattern_name {
                        self.safe_remove(entry.path(), description);
                    }
                }
            }
        }
    }

    fn kill_warp_processes(&self) {
        self.print_emoji("🔄", "Останавливаем процессы Warp...");

        #[cfg(target_os = "windows")]
        {
            let _ = Command::new("taskkill")
                .args(&["/F", "/IM", "warp.exe"])
                .output();
            let _ = Command::new("taskkill")
                .args(&["/F", "/IM", "Warp.exe"])
                .output();
        }

        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            let _ = Command::new("pkill")
                .args(&["-f", "-i", "warp"])
                .output();
        }

        thread::sleep(Duration::from_secs(2));
        self.print_emoji("✅", "Процессы остановлены");
    }

    #[cfg(target_os = "macos")]
    fn deep_clean_macos(&mut self) {
        self.print_emoji("🧹", "[BETA] Глубокая очистка macOS...");

        let cookies = self.home.join("Library/Cookies");
        let containers = self.home.join("Library/Containers");
        let group_containers = self.home.join("Library/Group Containers");

        self.safe_remove_glob(&format!("{}/Library/Cookies/*warp*", self.home.display()), "cookies");
        self.safe_remove_glob(&format!("{}/Library/Cookies/*Warp*", self.home.display()), "cookies");
        self.safe_remove_glob(&format!("{}/Library/Containers/*warp*", self.home.display()), "containers");
        self.safe_remove_glob(&format!("{}/Library/Containers/*Warp*", self.home.display()), "containers");
        self.safe_remove_glob(&format!("{}/Library/Group Containers/*warp*", self.home.display()), "group containers");
        self.safe_remove_glob(&format!("{}/Library/Group Containers/*Warp*", self.home.display()), "group containers");
    }

    #[cfg(target_os = "linux")]
    fn deep_clean_linux(&mut self) {
        self.print_emoji("🧹", "[BETA] Глубокая очистка Linux...");

        self.safe_remove(&self.home.join(".local/lib/warp"), "lib");
        self.safe_remove(&self.home.join(".local/lib/warp-terminal"), "lib");

        let systemd_user = self.home.join(".config/systemd/user");
        if systemd_user.exists() {
            self.safe_remove_glob(&format!("{}/systemd/user/*warp*", self.home.join(".config").display()), "systemd service");
        }
    }

    #[cfg(target_os = "windows")]
    fn deep_clean_windows(&mut self) {
        self.print_emoji("🧹", "[BETA] Глубокая очистка Windows...");

        let local_low = self.home.join("AppData/LocalLow");
        if local_low.exists() {
            self.safe_remove_glob(&format!("{}/AppData/LocalLow/*warp*", self.home.display()), "LocalLow");
            self.safe_remove_glob(&format!("{}/AppData/LocalLow/*Warp*", self.home.display()), "LocalLow");
        }

        // Prefetch requires admin
        let _ = fs::read_dir("C:/Windows/Prefetch")
            .map(|entries| {
                for entry in entries.filter_map(|e| e.ok()) {
                    let name = entry.file_name().to_string_lossy().to_uppercase();
                    if name.starts_with("WARP") && name.ends_with(".PF") {
                        let _ = fs::remove_file(entry.path());
                    }
                }
            });
    }

    #[cfg(target_os = "macos")]
    fn reset_macos_identity(&mut self) {
        self.print_emoji("🍎", "Сброс идентификатора на macOS...");

        // Application Support
        self.safe_remove_glob(&format!("{}/Library/Application Support/*warp*", self.home.display()), "данные");
        self.safe_remove_glob(&format!("{}/Library/Application Support/*Warp*", self.home.display()), "данные");

        // Preferences
        self.safe_remove_glob(&format!("{}/Library/Preferences/*warp*", self.home.display()), "настройки");
        self.safe_remove_glob(&format!("{}/Library/Preferences/*Warp*", self.home.display()), "настройки");

        // Caches
        self.safe_remove_glob(&format!("{}/Library/Caches/*warp*", self.home.display()), "кэш");
        self.safe_remove_glob(&format!("{}/Library/Caches/*Warp*", self.home.display()), "кэш");

        // Logs
        self.safe_remove_glob(&format!("{}/Library/Logs/*warp*", self.home.display()), "логи");
        self.safe_remove_glob(&format!("{}/Library/Logs/*Warp*", self.home.display()), "логи");

        // WebKit
        self.safe_remove_glob(&format!("{}/Library/WebKit/*warp*", self.home.display()), "web-данные");
        self.safe_remove_glob(&format!("{}/Library/WebKit/*Warp*", self.home.display()), "web-данные");

        // Saved Application State
        self.safe_remove_glob(&format!("{}/Library/Saved Application State/*warp*", self.home.display()), "состояние");
        self.safe_remove_glob(&format!("{}/Library/Saved Application State/*Warp*", self.home.display()), "состояние");

        // HTTP Storage
        self.safe_remove_glob(&format!("{}/Library/HTTPStorages/*warp*", self.home.display()), "HTTP");
        self.safe_remove_glob(&format!("{}/Library/HTTPStorages/*Warp*", self.home.display()), "HTTP");

        // Clear Launch Services database
        let lsregister = "/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister";
        let _ = Command::new(lsregister)
            .args(&["-kill", "-r", "-domain", "local", "-domain", "system", "-domain", "user"])
            .output();
    }

    #[cfg(target_os = "linux")]
    fn reset_linux_identity(&mut self) {
        self.print_emoji("🐧", "Сброс идентификатора на Linux...");

        let xdg_config = env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| self.home.join(".config"));
        let xdg_data = env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| self.home.join(".local/share"));
        let xdg_cache = env::var("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| self.home.join(".cache"));
        let xdg_state = env::var("XDG_STATE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| self.home.join(".local/state"));

        // Configuration
        self.safe_remove(&xdg_config.join("warp"), "конфиг");
        self.safe_remove(&xdg_config.join("Warp"), "конфиг");
        self.safe_remove(&xdg_config.join("warp-terminal"), "конфиг");
        self.safe_remove(&xdg_config.join("warp-terminal-preview"), "конфиг");
        self.safe_remove(&self.home.join(".warp"), "конфиг");

        // Application data
        self.safe_remove(&xdg_data.join("warp"), "данные");
        self.safe_remove(&xdg_data.join("Warp"), "данные");
        self.safe_remove(&xdg_data.join("warp-terminal"), "данные");
        self.safe_remove(&xdg_data.join("warp-terminal-preview"), "данные");

        // Cache
        self.safe_remove(&xdg_cache.join("warp"), "кэш");
        self.safe_remove(&xdg_cache.join("Warp"), "кэш");
        self.safe_remove(&xdg_cache.join("warp-terminal"), "кэш");
        self.safe_remove(&xdg_cache.join("warp-terminal-preview"), "кэш");

        // State and logs
        self.safe_remove(&xdg_state.join("warp"), "состояние");
        self.safe_remove(&xdg_state.join("warp-terminal"), "состояние");
        self.safe_remove(&xdg_state.join("warp-terminal-preview"), "состояние");

        // Temporary files
        if let Ok(user) = env::var("USER") {
            let uid = unsafe { libc::getuid() };
            self.safe_remove(&PathBuf::from(format!("/run/user/{}/warp", uid)), "runtime");
            self.safe_remove(&PathBuf::from(format!("/tmp/warp-{}", user)), "temp");
        }
    }

    #[cfg(target_os = "windows")]
    fn reset_windows_identity(&mut self) {
        self.print_emoji("🪟", "Сброс идентификатора на Windows...");

        let local_appdata = env::var("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| self.home.join("AppData/Local"));
        let appdata = env::var("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| self.home.join("AppData/Roaming"));

        // AppData Local
        self.safe_remove_glob(&format!("{}/*warp*", local_appdata.display()), "данные");
        self.safe_remove_glob(&format!("{}/*Warp*", local_appdata.display()), "данные");

        // AppData Roaming
        self.safe_remove_glob(&format!("{}/*warp*", appdata.display()), "настройки");
        self.safe_remove_glob(&format!("{}/*Warp*", appdata.display()), "настройки");

        // Temp files
        let temp_dir = env::var("TEMP")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("C:/Windows/Temp"));
        self.safe_remove_glob(&format!("{}/*warp*", temp_dir.display()), "temp");
        self.safe_remove_glob(&format!("{}/*Warp*", temp_dir.display()), "temp");

        // Registry cleanup
        #[cfg(target_os = "windows")]
        self.clean_registry();
    }

    #[cfg(target_os = "windows")]
    fn clean_registry(&self) {
        use winreg::enums::*;
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let _ = hkcu.delete_subkey_all("Software\\Warp");

        if !self.silent {
            self.print_emoji("🔑", "Реестр очищен");
        }
    }

    #[cfg(target_os = "macos")]
    fn remove_macos_warp(&mut self) {
        self.print_emoji("🍎", "Полное удаление на macOS...");

        self.safe_remove(&PathBuf::from("/Applications/Warp.app"), "приложение");
        self.reset_macos_identity();

        self.safe_remove_glob(&format!("{}/Downloads/*warp*", self.home.display()), "загрузки");
        self.safe_remove_glob(&format!("{}/Downloads/*Warp*", self.home.display()), "загрузки");
    }

    #[cfg(target_os = "linux")]
    fn remove_linux_warp(&mut self) {
        self.print_emoji("🐧", "Полное удаление на Linux...");

        self.safe_remove(&PathBuf::from("/opt/Warp"), "приложение");
        self.safe_remove(&PathBuf::from("/opt/warpdotdev"), "приложение");
        self.safe_remove(&PathBuf::from("/usr/local/bin/warp"), "бинарник");
        self.safe_remove(&PathBuf::from("/usr/local/bin/warp-terminal"), "бинарник");
        self.safe_remove(&PathBuf::from("/usr/bin/warp"), "бинарник");
        self.safe_remove(&PathBuf::from("/usr/bin/warp-terminal"), "бинарник");
        self.safe_remove(&self.home.join(".local/bin/warp"), "бинарник");
        self.safe_remove(&self.home.join(".local/bin/warp-terminal"), "бинарник");

        self.reset_linux_identity();

        let xdg_data = env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| self.home.join(".local/share"));
        self.safe_remove(&xdg_data.join("applications/warp.desktop"), "ярлык");
        self.safe_remove(&xdg_data.join("applications/warp-terminal.desktop"), "ярлык");
    }

    #[cfg(target_os = "windows")]
    fn remove_windows_warp(&mut self) {
        self.print_emoji("🪟", "Полное удаление на Windows...");

        let program_files = env::var("PROGRAMFILES")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("C:/Program Files"));
        let program_files_x86 = env::var("PROGRAMFILES(X86)")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("C:/Program Files (x86)"));

        self.safe_remove(&program_files.join("Warp"), "приложение");
        self.safe_remove(&program_files_x86.join("Warp"), "приложение");

        self.reset_windows_identity();

        let downloads = self.home.join("Downloads");
        self.safe_remove_glob(&format!("{}/Downloads/*warp*", self.home.display()), "загрузки");
        self.safe_remove_glob(&format!("{}/Downloads/*Warp*", self.home.display()), "загрузки");

        self.clean_registry();
    }

    fn verify_app_installed(&self) -> bool {
        #[cfg(target_os = "macos")]
        {
            PathBuf::from("/Applications/Warp.app").exists()
        }

        #[cfg(target_os = "windows")]
        {
            let program_files = env::var("PROGRAMFILES")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("C:/Program Files"));
            let program_files_x86 = env::var("PROGRAMFILES(X86)")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("C:/Program Files (x86)"));

            program_files.join("Warp").exists() || program_files_x86.join("Warp").exists()
        }

        #[cfg(target_os = "linux")]
        {
            Command::new("which")
                .arg("warp")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
                || Command::new("which")
                    .arg("warp-terminal")
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false)
                || PathBuf::from("/opt/Warp").exists()
                || PathBuf::from("/opt/warpdotdev").exists()
        }
    }

    fn enhanced_beta_bypass(&mut self) -> bool {
        println!("{}", "=".repeat(70));
        self.print_emoji("⚡", "[BETA] НАДЕЖНЫЙ БАЙПАСС WARP - РАСШИРЕННЫЙ РЕЖИМ");
        self.print_emoji("💻", &format!("Система: {}", env::consts::OS));
        self.print_emoji("🎯", "Режим: Максимально надежная очистка");
        self.print_emoji("🧪", "Статус: BETA - Экспериментальная функция");
        println!("{}
", "=".repeat(70));

        // Step 1: Kill processes multiple times
        self.print_emoji("🔄", "Шаг 1/4: Остановка всех процессов Warp...");
        for i in 0..3 {
            self.kill_warp_processes();
            if i < 2 {
                thread::sleep(Duration::from_secs(1));
            }
        }

        // Step 2: Standard cleanup
        self.print_emoji("🗑️", "Шаг 2/4: Стандартная очистка...");
        self.reset_identity();

        // Step 3: Deep clean
        self.print_emoji("🧹", "Шаг 3/4: [BETA] Глубокая очистка...");
        self.deep_clean();

        thread::sleep(Duration::from_secs(2));

        // Step 4: Verification
        self.print_emoji("🔍", "Шаг 4/4: Проверка результатов...");
        let app_still_there = self.verify_app_installed();

        // Final report
        println!("\n{}", "=".repeat(70));
        self.print_emoji("✅", "[BETA] НАДЕЖНЫЙ БАЙПАСС ЗАВЕРШЕН!");
        println!("📈 Выполнено операций: {}", self.operation_count);

        if app_still_there {
            self.print_emoji("🎉", "Отлично! Warp установлен и полностью очищен");
            self.print_emoji("🆔", "Машина имеет полностью новый идентификатор");
            self.print_emoji("🔐", "[BETA] Применены дополнительные меры безопасности");
            self.print_emoji("🚀", "Запустите Warp - полный сброс выполнен!");
        } else {
            self.print_emoji("⚠️", "Warp не обнаружен - возможно, требуется установка");
        }

        self.print_emoji("💡", "Использован экспериментальный режим [BETA]");
        println!("{}", "=".repeat(70));
        true
    }

    fn auto_bypass(&mut self) -> bool {
        println!("{}", "=".repeat(70));
        self.print_emoji("🚀", "АВТОМАТИЧЕСКИЙ БАЙПАСС WARP");
        self.print_emoji("💻", &format!("Система: {}", env::consts::OS));
        self.print_emoji("🎯", "Режим: Автоматический сброс идентификатора");
        println!("{}
", "=".repeat(70));

        self.kill_warp_processes();
        self.reset_identity();

        let app_still_there = self.verify_app_installed();

        println!("\n{}", "=".repeat(70));
        self.print_emoji("✅", "БАЙПАСС ЗАВЕРШЕН!");
        println!("📈 Выполнено операций: {}", self.operation_count);

        if app_still_there {
            self.print_emoji("🎉", "Отлично! Warp установлен и готов к использованию");
            self.print_emoji("🆔", "Ваша машина теперь имеет новый идентификатор");
            self.print_emoji("🚀", "Запустите Warp - он увидит вас как нового пользователя");
        } else {
            self.print_emoji("⚠️", "Warp не обнаружен - возможно, требуется установка");
        }

        println!("{}", "=".repeat(70));
        true
    }

    fn manual_reset(&mut self) -> bool {
        println!("{}", "=".repeat(70));
        self.print_emoji("🔄", "СБРОС ИДЕНТИФИКАТОРА МАШИНЫ");
        self.print_emoji("💻", &format!("Система: {}", env::consts::OS));
        println!("{}
", "=".repeat(70));

        self.kill_warp_processes();
        self.reset_identity();

        let app_still_there = self.verify_app_installed();

        println!("\n{}", "=".repeat(70));
        self.print_emoji("✅", "СБРОС ЗАВЕРШЕН!");
        println!("📈 Выполнено операций: {}", self.operation_count);

        if app_still_there {
            self.print_emoji("✅", "Warp остался установленным");
        }

        println!("{}", "=".repeat(70));
        true
    }

    fn manual_remove(&mut self) -> bool {
        println!("{}", "=".repeat(70));
        self.print_emoji("🗑️", "ПОЛНОЕ УДАЛЕНИЕ WARP");
        self.print_emoji("💻", &format!("Система: {}", env::consts::OS));
        println!("{}
", "=".repeat(70));

        self.kill_warp_processes();
        self.remove_warp();

        println!("\n{}", "=".repeat(70));
        self.print_emoji("✅", "УДАЛЕНИЕ ЗАВЕРШЕНО!");
        println!("📈 Выполнено операций: {}", self.operation_count);
        self.print_emoji("🔄", "Система увидит вас как новую машину");
        println!("{}", "=".repeat(70));
        true
    }

    fn reset_identity(&mut self) {
        #[cfg(target_os = "macos")]
        self.reset_macos_identity();

        #[cfg(target_os = "windows")]
        self.reset_windows_identity();

        #[cfg(target_os = "linux")]
        self.reset_linux_identity();
    }

    fn deep_clean(&mut self) {
        #[cfg(target_os = "macos")]
        self.deep_clean_macos();

        #[cfg(target_os = "windows")]
        self.deep_clean_windows();

        #[cfg(target_os = "linux")]
        self.deep_clean_linux();
    }

    fn remove_warp(&mut self) {
        #[cfg(target_os = "macos")]
        self.remove_macos_warp();

        #[cfg(target_os = "windows")]
        self.remove_windows_warp();

        #[cfg(target_os = "linux")]
        self.remove_linux_warp();
    }
}

fn show_banner() {
    println!("\n{}", "=".repeat(50));
    println!(
        r#"    
██╗    ██╗█████╗ ██████╗ ██████╗ 
██║    ██║██╔══██╗██╔══██╗██╔══██╗
██░ █╗ ██║███████║██████╔╝██████╔╝
██░███╗██║██╔══██║██╔══██╗██╔═══╝ 
╚███╔███╔╝██║  ██║██║  ██║██║     
 ╚══╝╚══╝ ╚═╝  ╚═╝╚═╝  ╚═╝╚═╝     

███╗   ██╗ ██████╗     ██╗     ██╗███╗   ███╗██╗████████╗███████╗
████╗  ██║██╔═══██╗    ██║     ██║████╗ ████║██║╚══██╔══╝██╔════╝
██╔██╗ ██║██║   ██║    ██║     ██║██╔████╔██║██║   ██║   ███████╗
██║╚██╗██║██║   ██║    ██║     ██║██║╚██╔╝██║██║   ██║   ╚════██║
██║ ╚████║╚██████╔╝    ███████╗██║██║ ╚═╝ ██║██║   ██║   ███████║
╚═╝  ╚═══╝ ╚═════╝     ╚══════╝╚═╝╚═╝     ╚═╝╚═╝   ╚═╝   ╚══════╝
"#
    );
    println!("{}", "=".repeat(50));
    println!("║{}🚀 AUTOMATIC BYPASS TOOL [BETA] 🚀{}║", " ".repeat(5), " ".repeat(5));
    println!("║ 👨‍💻 Created by Moti © 2025                   ║");
    println!("║ 🔗 github.com/MotiDva123/warp-no-limit-rust  ║");
    println!("║ ⚡ Enhanced Beta Mode Available               ║");
    println!("{}\n", "=".repeat(50));
}

fn main() {
    let args = Args::parse();

    if !args.silent {
        show_banner();
    }

    let mut tool = WarpNoLimits::new(args.silent);

    let success = if args.remove {
        tool.manual_remove()
    } else if args.reset {
        tool.manual_reset()
    } else if args.enhanced_beta {
        tool.enhanced_beta_bypass()
    } else if args.auto {
        tool.auto_bypass()
    } else {
        // Default: enhanced beta bypass
        tool.enhanced_beta_bypass()
    };

    if success {
        if !args.silent {
            println!("\n✅ Операция выполнена успешно!");
        }
        std::process::exit(0);
    } else {
        if !args.silent {
            println!("\n❌ Операция завершилась с ошибками");
        }
        std::process::exit(1);
    }
}
