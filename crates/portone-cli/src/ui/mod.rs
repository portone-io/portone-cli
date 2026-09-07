pub mod pager;

use std::cell::RefCell;
use std::io::{IsTerminal, Write};
use std::rc::Rc;

pub struct IoStreams {
    pub out: Box<dyn Write>,
    pub err: Box<dyn Write>,
    pub stdout_is_tty: bool,
    no_color: bool,
    color_forced: bool,
}

impl IoStreams {
    pub fn detect() -> Self {
        Self {
            out: Box::new(std::io::stdout()),
            err: Box::new(std::io::stderr()),
            stdout_is_tty: std::io::stdout().is_terminal(),
            no_color: std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty()),
            color_forced: std::env::var_os("CLICOLOR_FORCE")
                .is_some_and(|v| !v.is_empty() && v != "0"),
        }
    }

    pub fn test() -> (Self, TestBuffers) {
        let buffers = TestBuffers {
            out: SharedBuf::default(),
            err: SharedBuf::default(),
        };
        let io = Self {
            out: Box::new(buffers.out.clone()),
            err: Box::new(buffers.err.clone()),
            stdout_is_tty: false,
            no_color: false,
            color_forced: false,
        };
        (io, buffers)
    }

    pub fn color_enabled(&self) -> bool {
        self.color_forced || (self.stdout_is_tty && !self.no_color)
    }
}

#[derive(Clone, Default)]
pub struct SharedBuf(Rc<RefCell<Vec<u8>>>);

impl Write for SharedBuf {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub struct TestBuffers {
    pub out: SharedBuf,
    pub err: SharedBuf,
}

impl TestBuffers {
    pub fn out(&self) -> String {
        String::from_utf8_lossy(&self.out.0.borrow()).into_owned()
    }

    pub fn err(&self) -> String {
        String::from_utf8_lossy(&self.err.0.borrow()).into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn streams(stdout_is_tty: bool, no_color: bool, color_forced: bool) -> IoStreams {
        IoStreams {
            out: Box::new(Vec::new()),
            err: Box::new(Vec::new()),
            stdout_is_tty,
            no_color,
            color_forced,
        }
    }

    #[test]
    fn color_enabled_when_tty_without_flags() {
        assert!(streams(true, false, false).color_enabled());
    }

    #[test]
    fn color_disabled_when_not_tty() {
        assert!(!streams(false, false, false).color_enabled());
    }

    #[test]
    fn color_disabled_by_no_color() {
        assert!(!streams(true, true, false).color_enabled());
    }

    #[test]
    fn clicolor_force_overrides_non_tty() {
        assert!(streams(false, false, true).color_enabled());
    }

    #[test]
    fn clicolor_force_overrides_no_color() {
        assert!(streams(true, true, true).color_enabled());
    }

    #[test]
    fn detect_runs() {
        let io = IoStreams::detect();
        let _ = io.color_enabled();
    }

    #[test]
    fn test_streams_capture_writes() {
        let (mut io, bufs) = IoStreams::test();
        io.out.write_all("standard output".as_bytes()).unwrap();
        io.err.write_all("standard error".as_bytes()).unwrap();
        assert_eq!(bufs.out(), "standard output");
        assert_eq!(bufs.err(), "standard error");
        assert!(!io.color_enabled());
    }
}
