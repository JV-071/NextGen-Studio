use crate::document::{Result, atomic_write, read_bounded};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
pub struct Preview {
    child: Option<Child>,
    session: Option<tempfile::TempDir>,
    pub status: String,
    pub revision: u64,
    last_poll: Instant,
    last_image: Option<std::time::SystemTime>,
}
impl Default for Preview {
    fn default() -> Self {
        Self {
            child: None,
            session: None,
            status: "Prévia nativa parada".into(),
            revision: 0,
            last_poll: Instant::now(),
            last_image: None,
        }
    }
}
impl Preview {
    pub fn start(&mut self, root: &Path, exe: &Path) -> Result<()> {
        if self.child.is_some() {
            return Ok(());
        }
        if !root.join("init.lua").is_file() {
            return Err("Escolha a raiz do NextGen (pasta com init.lua).".into());
        }
        let bridge = root.join("modules/dev_studio_bridge");
        fs::create_dir_all(&bridge).map_err(|e| e.to_string())?;
        for (name, data) in [
            (
                "bridge.lua",
                include_bytes!("../integration/dev_studio_bridge/bridge.lua").as_slice(),
            ),
            (
                "dev_studio_bridge.otmod",
                include_bytes!("../integration/dev_studio_bridge/dev_studio_bridge.otmod")
                    .as_slice(),
            ),
        ] {
            let p = bridge.join(name);
            if p.exists() && read_bounded(&p)? != data {
                return Err(format!(
                    "Integração existente difere da versão Rust. Preserve suas mudanças e atualize manualmente: {}",
                    p.display()
                ));
            }
            if !p.exists() {
                atomic_write(&p, data)?;
            }
        }
        let session = tempfile::Builder::new()
            .prefix("nextgen-studio-")
            .tempdir()
            .map_err(|e| e.to_string())?;
        let output = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(session.path().join("client.log"))
            .map_err(|e| e.to_string())?;
        let child = Command::new(exe)
            .current_dir(root)
            .env("NEXTGEN_STUDIO_SESSION_DIR", session.path())
            .stdout(Stdio::from(output.try_clone().map_err(|e| e.to_string())?))
            .stderr(Stdio::from(output))
            .spawn()
            .map_err(|e| e.to_string())?;
        self.child = Some(child);
        self.session = Some(session);
        self.status = "Cliente iniciado; aguardando integração…".into();
        log::info!("{}", self.status);
        Ok(())
    }
    pub fn running(&self) -> bool {
        self.child.is_some()
    }
    pub fn send(&mut self, text: &str) -> Result<()> {
        let session = self.session.as_ref().ok_or("Inicie a prévia nativa.")?;
        self.revision += 1;
        let req = serde_json::json!({"revision":self.revision,"text":text});
        atomic_write(
            &session.path().join("request.json"),
            &serde_json::to_vec(&req).unwrap(),
        )?;
        self.status = format!("Revisão {} enviada; aguardando motor", self.revision);
        Ok(())
    }
    pub fn poll(&mut self) -> Option<PathBuf> {
        if self.last_poll.elapsed() < Duration::from_millis(350) {
            return None;
        }
        self.last_poll = Instant::now();
        if let Some(child) = &mut self.child {
            if let Ok(Some(exit)) = child.try_wait() {
                self.status = format!("Cliente encerrado: {exit}");
                self.child = None;
            }
        }
        let session = self.session.as_ref()?;
        if let Ok(raw) = read_bounded(&session.path().join("response.json")) {
            if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&raw) {
                if value["revision"].as_u64() == Some(self.revision) {
                    self.status = value["message"]
                        .as_str()
                        .unwrap_or("Resposta recebida")
                        .to_owned();
                }
            }
        }
        let p = session.path().join("preview.png");
        let changed = fs::metadata(&p).ok()?.modified().ok()?;
        if self.last_image == Some(changed) {
            return None;
        }
        self.last_image = Some(changed);
        Some(p)
    }
    pub fn stop(&mut self) {
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
        self.status = "Prévia nativa parada".into();
        self.session = None;
        self.last_image = None;
    }
}
impl Drop for Preview {
    fn drop(&mut self) {
        self.stop();
    }
}
