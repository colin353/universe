//! This crate can be used for generally interacting with a tty's mode safely, but was
//! created originally to solve the problem of using raw mode with /dev/tty while reading
//! stdin for data.
//!
//! # Raw Mode
//!
//! Description from the `termion` crate:
//! >Managing raw mode.
//!
//! >Raw mode is a particular state a TTY can have. It signifies that:
//!
//! >1. No line buffering (the input is given byte-by-byte).
//! >2. The input is not written out, instead it has to be done manually by the programmer.
//! >3. The output is not canonicalized (for example, `\n` means "go one line down", not "line
//! >   break").
//!
//! >It is essential to design terminal programs.
//!
//! ## Example
//!
//! ```no_run
//! use raw_tty::IntoRawMode;
//! use std::io::{Write, stdin, stdout};
//!
//! fn main() {
//!     let stdin = stdin().into_raw_mode().unwrap();
//!     let mut stdout = stdout();
//!
//!     write!(stdout, "Hey there.").unwrap();
//! }
//! ```
//!
//! ## Example with /dev/tty
//!
//! ```
//! use raw_tty::IntoRawMode;
//! use std::io::{self, Read, Write, stdin, stdout};
//! use std::fs;
//!
//! fn main() -> io::Result<()> {
//!     let mut tty = fs::OpenOptions::new().read(true).write(true).open("/dev/tty")?;
//!     // Can use the tty_input for keys while also reading stdin for data.
//!     let mut tty_input = tty.try_clone()?.into_raw_mode();
//!     let mut buffer = String::new();
//!     stdin().read_to_string(&mut buffer)?;
//!
//!     write!(tty, "Hey there.")
//! }
//! ```
//!
//! # General example
//!
//! ```no_run
//! use raw_tty::GuardMode;
//! use std::io::{self, Write, stdin, stdout};
//!
//! fn test_into_raw_mode() -> io::Result<()> {
//!     let mut stdin = stdin().guard_mode()?;
//!     stdin.set_raw_mode()?;
//!     let mut out = stdout();
//!
//!     out.write_all(b"this is a test, muahhahahah\r\n")?;
//!
//!     drop(out);
//!     Ok(())
//! }
//!
//! fn main() -> io::Result<()> {
//!     let mut stdout = stdout().guard_mode()?;
//!     stdout.modify_mode(|ios| /* do stuff with termios here */ ios)?;
//!
//!     // Have to use &* since TtyModeGuard only implements
//!     // deref, unlike RawReader which implements read specifically.
//!     // Otherwise, it wouldn't be recognized as `Write`able.
//!     write!(&mut *stdout, "Hey there.")
//! }
//!
//! ```

mod util {
    use std::io;

    pub trait IsMinusOne {
        fn is_minus_one(&self) -> bool;
    }

    macro_rules! impl_is_minus_one {
            ($($t:ident)*) => ($(impl IsMinusOne for $t {
                fn is_minus_one(&self) -> bool {
                    *self == -1
                }
            })*)
        }

    impl_is_minus_one! { i8 i16 i32 i64 isize }

    pub fn convert_to_result<T: IsMinusOne>(t: T) -> io::Result<T> {
        if t.is_minus_one() {
            Err(io::Error::last_os_error())
        } else {
            Ok(t)
        }
    }
}

mod attr {
    #[cfg(unix)]
    pub mod unix {
        use crate::util::*;

        use libc::c_int;

        /// Export of libc::termios
        pub type Termios = libc::termios;

        use std::os::unix::io::RawFd;
        use std::{io, mem};

        pub fn get_terminal_attr(fd: RawFd) -> io::Result<Termios> {
            extern "C" {
                pub fn tcgetattr(fd: c_int, termptr: *mut Termios) -> c_int;
            }
            unsafe {
                let mut termios = mem::zeroed();
                convert_to_result(tcgetattr(fd, &mut termios))?;
                Ok(termios)
            }
        }

        pub fn set_terminal_attr(fd: RawFd, termios: &Termios) -> io::Result<()> {
            extern "C" {
                pub fn tcsetattr(fd: c_int, opt: c_int, termptr: *const Termios) -> c_int;
            }
            convert_to_result(unsafe { tcsetattr(fd, 0, termios) }).and(Ok(()))
        }

        pub fn raw_terminal_attr(termios: &mut Termios) {
            extern "C" {
                pub fn cfmakeraw(termptr: *mut Termios);
            }
            unsafe { cfmakeraw(termios) }
        }
    }

    #[cfg(unix)]
    pub use unix::*;
}

/// Export of libc::termios
pub use attr::Termios;

use attr::{get_terminal_attr, raw_terminal_attr, set_terminal_attr};
use std::io;
use std::os::unix::io::{AsRawFd, RawFd};

/// A terminal restorer, which keeps the previous state of the terminal, and restores it, when
/// dropped.
///
/// Restoring will entirely bring back the old TTY state.
pub struct TtyModeGuard {
    ios: Termios,
    fd: RawFd,
}

impl Drop for TtyModeGuard {
    fn drop(&mut self) {
        set_terminal_attr(self.fd, &self.ios).unwrap();
    }
}

impl TtyModeGuard {
    pub fn new(fd: RawFd) -> io::Result<TtyModeGuard> {
        let ios = get_terminal_attr(fd)?;

        Ok(Self { ios, fd })
    }

    /// Switch to raw mode.
    pub fn set_raw_mode(&mut self) -> io::Result<()> {
        let mut ios = self.ios;

        raw_terminal_attr(&mut ios);

        set_terminal_attr(self.fd, &ios)?;
        Ok(())
    }

    /// Creates a copy of the saved termios and passes it to `f`
    /// which should return the new termios to apply.
    ///
    /// This method can be used to restore the saved ios afterwards
    /// by using the identity function.
    pub fn modify_mode<F>(&mut self, f: F) -> io::Result<()>
    where
        F: FnOnce(Termios) -> Termios,
    {
        let ios = f(self.ios);
        set_terminal_attr(self.fd, &ios)?;
        Ok(())
    }
}

use std::io::Read;
use std::ops;

/// Wraps a file descriptor for a TTY with a guard which saves
/// the terminal mode on creation and restores it on drop.
pub struct TtyWithGuard<T: AsRawFd> {
    guard: TtyModeGuard,
    inner: T,
}

impl<R: AsRawFd> ops::Deref for TtyWithGuard<R> {
    type Target = R;

    #[inline]
    fn deref(&self) -> &R {
        &self.inner
    }
}

impl<R: AsRawFd> ops::DerefMut for TtyWithGuard<R> {
    #[inline]
    fn deref_mut(&mut self) -> &mut R {
        &mut self.inner
    }
}

impl<T: AsRawFd> TtyWithGuard<T> {
    pub fn new(tty: T) -> io::Result<TtyWithGuard<T>> {
        Ok(Self {
            guard: TtyModeGuard::new(tty.as_raw_fd())?,
            inner: tty,
        })
    }

    /// Creates a copy of the saved termios and passes it to `f`
    /// which should return the new termios to apply.
    ///
    /// This method can be used to restore the saved ios afterwards
    /// by using the identity function.
    pub fn modify_mode<F>(&mut self, f: F) -> io::Result<()>
    where
        F: FnOnce(Termios) -> Termios,
    {
        self.guard.modify_mode(f)
    }

    /// Switch to raw mode.
    pub fn set_raw_mode(&mut self) -> io::Result<()> {
        self.guard.set_raw_mode()
    }
}

/// Types which can save a termios.
pub trait GuardMode: AsRawFd + Sized {
    fn guard_mode(self) -> io::Result<TtyWithGuard<Self>>;
}

impl<T: AsRawFd> GuardMode for T {
    fn guard_mode(self) -> io::Result<TtyWithGuard<T>> {
        TtyWithGuard::new(self)
    }
}

impl<R: io::Read + AsRawFd> io::Read for TtyWithGuard<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

impl<R: io::Write + AsRawFd> io::Write for TtyWithGuard<R> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

pub struct RawReader<T: Read + AsRawFd>(TtyWithGuard<T>);

impl<T> ops::Deref for RawReader<T>
where
    T: Read + AsRawFd,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> ops::DerefMut for RawReader<T>
where
    T: Read + AsRawFd,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<R: Read + AsRawFd> Read for RawReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

/// Types which can be converted into "raw mode".
///
pub trait IntoRawMode: Read + AsRawFd + Sized {
    /// Switch to raw mode.
    ///
    /// Raw mode means that stdin won't be printed (it will instead have to be written manually by
    /// the program). Furthermore, the input isn't canonicalised or buffered (that is, you can
    /// read from stdin one byte of a time). The output is neither modified in any way.
    fn into_raw_mode(self) -> io::Result<RawReader<Self>>;
}

impl<T: Read + AsRawFd> IntoRawMode for T {
    fn into_raw_mode(self) -> io::Result<RawReader<T>> {
        let mut x = TtyWithGuard::new(self)?;
        x.set_raw_mode()?;
        Ok(RawReader(x))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io::{self, stdin, stdout, Write};
    use std::os::unix::io::AsRawFd;

    #[test]
    fn test_into_raw_mode() -> io::Result<()> {
        let mut stdin = stdin().guard_mode()?;
        stdin.set_raw_mode()?;
        let mut out = stdout();

        out.write_all(b"testing, 123\r\n")?;

        drop(out);
        Ok(())
    }
}
