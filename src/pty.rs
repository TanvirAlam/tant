// PTY + process management
// Spawn the user's shell inside a pseudo-terminal

use pty::{fork, PtyMaster};
use std::process::Command;

pub struct PtyManager {
    pty_master: PtyMaster,
    child: std::process::Child,
}

impl PtyManager {
    pub fn new(shell: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let fork = fork()?;
        let mut cmd = Command::new(shell);
        cmd.stdin(fork.slave.try_clone()?);
        cmd.stdout(fork.slave.try_clone()?);
        cmd.stderr(fork.slave.try_clone()?);
        cmd.cwd(std::env::current_dir()?);
        let child = cmd.spawn()?;
        Ok(PtyManager {
            pty_master: fork.master,
            child,
        })
    }

    pub fn reader(&self) -> &dyn std::io::Read {
        &self.pty_master
    }

    pub fn writer(&self) -> &dyn std::io::Write {
        &self.pty_master
    }

    pub fn resize(&self, rows: u16, cols: u16) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: resize
        Ok(())
    }
}