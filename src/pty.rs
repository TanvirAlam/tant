// PTY + process management
// Spawn the user's shell inside a pseudo-terminal

// use portable_pty::{CommandBuilder, PtySize};
// #[cfg(unix)]
// use portable_pty::unix::PtySystem as ConcretePtySystem;
// #[cfg(windows)]
// use portable_pty::windows::PtySystem as ConcretePtySystem;
use tokio::io::{AsyncRead, AsyncWrite};

pub struct _PtyManager {
    master: Box<dyn portable_pty::MasterPty + Send>,
    child: Box<dyn portable_pty::Child + Send>,
    reader: Box<dyn AsyncRead + Send + Unpin>,
    writer: Box<dyn AsyncWrite + Send + Unpin>,
}

/*
impl PtyManager {
    pub fn new(shell: &str) -> Result<Self, Box<dyn std::error::Error>> {
        #[cfg(unix)]
        let pty_system = PtySystem::default();
        #[cfg(windows)]
        let pty_system = ConcretePtySystem::default();
        let pair = pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        let mut cmd = CommandBuilder::new(shell);
        cmd.cwd(std::env::current_dir()?);
        let child = pair.slave.spawn_command(cmd)?;
        let reader = pair.master.input;
        let writer = pair.master.output;
        Ok(PtyManager {
            master: pair.master,
            child,
            reader,
            writer,
        })
    }

    pub fn reader(&mut self) -> &mut (dyn AsyncRead + Send + Unpin) {
        &mut *self.reader
    }

    pub fn writer(&mut self) -> &mut (dyn AsyncWrite + Send + Unpin) {
        &mut *self.writer
    }

    pub fn resize(&mut self, rows: u16, cols: u16) -> Result<(), Box<dyn std::error::Error>> {
        self.master.resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })?;
        Ok(())
    }
}
*/