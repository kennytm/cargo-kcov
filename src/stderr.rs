//! Wrapper of `term::stderr` which fallbacks to colorless output if disabled.

use std::io;
use std::fmt::Arguments;

use term::{stderr, Terminal, StderrTerminal, Attr, Result, Error};
use term::color::Color;

/// Creates a new stderr console, which is capable of coloring, and gracefully fallback to colorless
/// output if stderr does not support it.
pub fn new() -> Box<StderrTerminal> {
    stderr().unwrap_or_else(|| Box::new(ColorlessWriter(io::stderr())))
}

/// Wraps a writer which implements `term::Terminal` which ignores all styling commands. This
/// structure is used when `term::stderr()` returns None when targeting non-TTY.
struct ColorlessWriter<W: io::Write>(W);

impl<W: io::Write> Terminal for ColorlessWriter<W> {
    type Output = W;

    fn fg(&mut self, _: Color) -> Result<()> { Ok(()) }
    fn bg(&mut self, _: Color) -> Result<()> { Ok(()) }
    fn attr(&mut self, _: Attr) -> Result<()> { Ok(()) }
    fn supports_attr(&self, _: Attr) -> bool { true }
    fn reset(&mut self) -> Result<()> { Ok(()) }
    fn supports_reset(&self) -> bool { true }
    fn supports_color(&self) -> bool { true }
    fn cursor_up(&mut self) -> Result<()> { Err(Error::NotSupported) }
    fn delete_line(&mut self) -> Result<()> { Err(Error::NotSupported) }
    fn carriage_return(&mut self) -> Result<()> { Err(Error::NotSupported) }
    fn get_ref(&self) -> &Self::Output { &self.0 }
    fn get_mut(&mut self) -> &mut Self::Output { &mut self.0 }
    fn into_inner(self) -> Self::Output { self.0 }
}

impl<W: io::Write> io::Write for ColorlessWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> { self.0.write(buf) }
    fn flush(&mut self) -> io::Result<()> { self.0.flush() }
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> { self.0.write_all(buf) }
    fn write_fmt(&mut self, fmt: Arguments) -> io::Result<()> { self.0.write_fmt(fmt) }
}
