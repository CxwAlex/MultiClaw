//! 实例服务管理
//! 提供将实例注册为系统服务（systemd/launchd）的功能

use std::path::PathBuf;
use anyhow::Result;

/// 实例服务管理器
pub struct InstanceService {
    /// 实例 ID
    instance_id: String,
    /// 服务名称
    service_name: String,
    /// 是否是董事长实例
    is_chairman: bool,
}

impl InstanceService {
    /// 创建新的实例服务管理器
    pub fn new(instance_id: &str, is_chairman: bool) -> Self {
        let service_name = if is_chairman {
            "multiclaw-chairman".to_string()
        } else {
            format!("multiclaw-{}", &instance_id[..8])
        };
        
        Self {
            instance_id: instance_id.to_string(),
            service_name,
            is_chairman,
        }
    }

    /// 注册实例为系统服务
    pub async fn register(&self, exe_path: &PathBuf, config_dir: &PathBuf, port: u16) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            self.register_systemd(exe_path, config_dir, port).await?;
        }
        
        #[cfg(target_os = "macos")]
        {
            self.register_launchd(exe_path, config_dir, port).await?;
        }
        
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            tracing::warn!("Service registration not supported on this platform");
        }
        
        Ok(())
    }

    /// 取消注册服务
    pub async fn unregister(&self) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            self.unregister_systemd().await?;
        }
        
        #[cfg(target_os = "macos")]
        {
            self.unregister_launchd().await?;
        }
        
        Ok(())
    }

    /// 启动服务
    pub async fn start(&self) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            tokio::process::Command::new("systemctl")
                .args(&["--user", "start", &self.service_name])
                .status()
                .await?;
        }
        
        #[cfg(target_os = "macos")]
        {
            let plist_path = self.launchd_plist_path();
            tokio::process::Command::new("launchctl")
                .args(&["load", "-w", plist_path.to_str().unwrap()])
                .status()
                .await?;
        }
        
        Ok(())
    }

    /// 停止服务
    pub async fn stop(&self) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            tokio::process::Command::new("systemctl")
                .args(&["--user", "stop", &self.service_name])
                .status()
                .await?;
        }
        
        #[cfg(target_os = "macos")]
        {
            let plist_path = self.launchd_plist_path();
            tokio::process::Command::new("launchctl")
                .args(&["unload", plist_path.to_str().unwrap()])
                .status()
                .await?;
        }
        
        Ok(())
    }

    /// 检查服务是否在运行
    pub async fn is_running(&self) -> bool {
        #[cfg(target_os = "linux")]
        {
            let result = tokio::process::Command::new("systemctl")
                .args(&["--user", "is-active", &self.service_name])
                .status()
                .await;
            
            result.map(|s| s.success()).unwrap_or(false)
        }
        
        #[cfg(target_os = "macos")]
        {
            let result = tokio::process::Command::new("launchctl")
                .args(&["list", &self.service_name])
                .status()
                .await;
            
            result.map(|s| s.success()).unwrap_or(false)
        }
        
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            false
        }
    }

    #[cfg(target_os = "linux")]
    fn systemd_service_path(&self) -> PathBuf {
        directories::UserDirs::new()
            .map(|dirs| dirs.home_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".config")
            .join("systemd")
            .join("user")
            .join(format!("{}.service", self.service_name))
    }

    #[cfg(target_os = "macos")]
    fn launchd_plist_path(&self) -> PathBuf {
        directories::UserDirs::new()
            .map(|dirs| dirs.home_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Library")
            .join("LaunchAgents")
            .join(format!("{}.plist", self.service_name))
    }

    #[cfg(target_os = "linux")]
    async fn register_systemd(&self, exe_path: &PathBuf, config_dir: &PathBuf, port: u16) -> Result<()> {
        let service_content = format!(
            r#"[Unit]
Description=MultiClaw Instance - {}
After=network.target

[Service]
Type=simple
ExecStart={} daemon --config-dir {} --port {}
Restart=always
RestartSec=3
Environment=MULTICLAW_INSTANCE_ID={}

[Install]
WantedBy=default.target
"#,
            self.instance_id,
            exe_path.display(),
            config_dir.display(),
            port,
            self.instance_id
        );

        let service_path = self.systemd_service_path();
        
        if let Some(parent) = service_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        tokio::fs::write(&service_path, service_content).await?;
        
        // 重新加载 systemd
        let _ = tokio::process::Command::new("systemctl")
            .args(&["--user", "daemon-reload"])
            .status()
            .await;
        
        // 启用服务
        let _ = tokio::process::Command::new("systemctl")
            .args(&["--user", "enable", &self.service_name])
            .status()
            .await;
        
        tracing::info!("Registered systemd service: {}", self.service_name);
        Ok(())
    }

    #[cfg(target_os = "linux")]
    async fn unregister_systemd(&self) -> Result<()> {
        // 停止并禁用服务
        let _ = tokio::process::Command::new("systemctl")
            .args(&["--user", "stop", &self.service_name])
            .status()
            .await;
        
        let _ = tokio::process::Command::new("systemctl")
            .args(&["--user", "disable", &self.service_name])
            .status()
            .await;
        
        // 删除服务文件
        let service_path = self.systemd_service_path();
        if service_path.exists() {
            tokio::fs::remove_file(&service_path).await?;
        }
        
        // 重新加载 systemd
        let _ = tokio::process::Command::new("systemctl")
            .args(&["--user", "daemon-reload"])
            .status()
            .await;
        
        tracing::info!("Unregistered systemd service: {}", self.service_name);
        Ok(())
    }

    #[cfg(target_os = "macos")]
    async fn register_launchd(&self, exe_path: &PathBuf, config_dir: &PathBuf, port: u16) -> Result<()> {
        let plist_content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
        <string>daemon</string>
        <string>--config-dir</string>
        <string>{}</string>
        <string>--port</string>
        <string>{}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/tmp/multiclaw-{}.log</string>
    <key>StandardErrorPath</key>
    <string>/tmp/multiclaw-{}.error.log</string>
    <key>EnvironmentVariables</key>
    <dict>
        <key>MULTICLAW_INSTANCE_ID</key>
        <string>{}</string>
    </dict>
</dict>
</plist>
"#,
            self.service_name,
            exe_path.display(),
            config_dir.display(),
            port,
            self.instance_id,
            self.instance_id,
            self.instance_id
        );

        let plist_path = self.launchd_plist_path();
        
        if let Some(parent) = plist_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        tokio::fs::write(&plist_path, plist_content).await?;
        
        // 加载服务
        let _ = tokio::process::Command::new("launchctl")
            .args(&["load", "-w", plist_path.to_str().unwrap()])
            .status()
            .await;
        
        tracing::info!("Registered launchd service: {}", self.service_name);
        Ok(())
    }

    #[cfg(target_os = "macos")]
    async fn unregister_launchd(&self) -> Result<()> {
        let plist_path = self.launchd_plist_path();
        
        // 卸载服务
        let _ = tokio::process::Command::new("launchctl")
            .args(&["unload", plist_path.to_str().unwrap()])
            .status()
            .await;
        
        // 删除 plist 文件
        if plist_path.exists() {
            tokio::fs::remove_file(&plist_path).await?;
        }
        
        tracing::info!("Unregistered launchd service: {}", self.service_name);
        Ok(())
    }
}

/// 列出所有 MultiClaw 服务
pub async fn list_services() -> Vec<String> {
    let mut services = Vec::new();
    
    #[cfg(target_os = "linux")]
    {
        let result = tokio::process::Command::new("systemctl")
            .args(&["--user", "list-units", "--all", "--type=service", "--plain"])
            .output()
            .await;
        
        if let Ok(output) = result {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("multiclaw") {
                    if let Some(name) = line.split_whitespace().next() {
                        services.push(name.to_string());
                    }
                }
            }
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        let result = tokio::process::Command::new("launchctl")
            .args(&["list"])
            .output()
            .await;
        
        if let Ok(output) = result {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("multiclaw") {
                    if let Some(name) = line.split_whitespace().nth(2) {
                        services.push(name.to_string());
                    }
                }
            }
        }
    }
    
    services
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_name_generation() {
        let service = InstanceService::new("company_abc123", false);
        assert!(service.service_name.starts_with("multiclaw-"));
        
        let chairman_service = InstanceService::new("chairman", true);
        assert_eq!(chairman_service.service_name, "multiclaw-chairman");
    }
}