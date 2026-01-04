// PTY + process management
// Spawn the user's shell inside a pseudo-terminal

use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{Read, Write};
use nix::fcntl::{fcntl, FcntlArg, OFlag};

pub struct PtyManager {
    master: Box<dyn portable_pty::MasterPty + Send>,
    #[allow(dead_code)]
    child: Box<dyn portable_pty::Child + Send>,
    reader: Box<dyn Read + Send>,
    writer: Box<dyn Write + Send>,
}

impl PtyManager {
    pub fn new(shell: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Self::new_with_cwd(shell, std::env::current_dir()?)
    }

    pub fn new_with_cwd(shell: &str, cwd: std::path::PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let pty_system = native_pty_system();
        let pair = pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        
        // Set the master PTY to non-blocking mode
        if let Some(fd) = pair.master.as_raw_fd() {
            let flags = fcntl(fd, FcntlArg::F_GETFL)?;
            let oflags = OFlag::from_bits_truncate(flags);
            fcntl(fd, FcntlArg::F_SETFL(oflags | OFlag::O_NONBLOCK))?;
        }
        
        let mut cmd = CommandBuilder::new(shell);
        cmd.cwd(cwd);
        let child = pair.slave.spawn_command(cmd)?;
        let reader = pair.master.try_clone_reader()?;
        let writer = pair.master.take_writer()?;
        Ok(PtyManager {
            master: pair.master,
            child,
            reader,
            writer,
        })
    }

    pub fn reader(&mut self) -> &mut (dyn Read + Send) {
        &mut self.reader
    }

    pub fn writer(&mut self) -> &mut (dyn Write + Send) {
        &mut self.writer
    }

    pub fn resize(&mut self, rows: u16, cols: u16, pixel_width: u16, pixel_height: u16) -> Result<(), Box<dyn std::error::Error>> {
        self.master.resize(PtySize { rows, cols, pixel_width, pixel_height })?;
        Ok(())
    }

    pub fn spawn_reader(&self, sender: tokio::sync::mpsc::Sender<Vec<u8>>) {
        let mut reader = self.master.try_clone_reader().expect("Failed to clone reader");
        tokio::spawn(async move {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        let data = buf[..n].to_vec();
                        if sender.send(data).await.is_err() {
                            break; // Receiver dropped
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                    }
                    Err(_) => break,
                }
            }
        });
    }
}