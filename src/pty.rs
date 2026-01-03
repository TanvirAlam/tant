// PTY + process management
// Spawn the user's shell inside a pseudo-terminal

use portable_pty::{native_pty_system, CommandBuilder, PtySize, PtyPair, Child};

pub struct PtyManager {
    pty_pair: PtyPair,
    child: Box<dyn Child>,
}

impl PtyManager {
    pub fn new(shell: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let pty_system = native_pty_system();
        let pty_pair = pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        let mut cmd = CommandBuilder::new(shell);
        cmd.cwd(std::env::current_dir()?);
        let child = pty_pair.slave.spawn_command(cmd)?;
        Ok(PtyManager { pty_pair, child })
    }

    pub fn reader(&self) -> &dyn std::io::Read {
        &*self.pty_pair.master
    }

    pub fn writer(&self) -> &dyn std::io::Write {
        &*self.pty_pair.master
    }

    pub fn resize(&self, rows: u16, cols: u16) -> Result<(), Box<dyn std::error::Error>> {
        self.pty_pair.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        Ok(())
    }
}