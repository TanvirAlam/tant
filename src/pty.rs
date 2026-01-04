// PTY + process management
// Spawn the user's shell inside a pseudo-terminal

use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{BufReader, BufWriter, Read, Write};

pub struct PtyManager {
    master: Box<dyn portable_pty::MasterPty + Send>,
    #[allow(dead_code)]
    child: Box<dyn portable_pty::Child + Send>,
    reader: BufReader<Box<dyn Read + Send>>,
    writer: BufWriter<Box<dyn Write + Send>>,
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
        let mut cmd = CommandBuilder::new(shell);
        cmd.cwd(cwd);
        let child = pair.slave.spawn_command(cmd)?;
        let reader = BufReader::new(pair.master.try_clone_reader()?);
        let writer = BufWriter::new(pair.master.take_writer()?);
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

    pub fn resize(&mut self, rows: u16, cols: u16) -> Result<(), Box<dyn std::error::Error>> {
        self.master.resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })?;
        Ok(())
    }
}