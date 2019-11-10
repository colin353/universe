use ffi;
use libc::c_int;
use std::error;
use std::error::Error as StdError;
use std::fmt;
use std::io;

use error::ErrorStack;
use ssl::MidHandshakeSslStream;
use x509::X509VerifyResult;

/// An error code returned from SSL functions.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ErrorCode(c_int);

impl ErrorCode {
    pub fn from_raw(raw: c_int) -> ErrorCode {
        ErrorCode(raw)
    }

    pub fn as_raw(&self) -> c_int {
        self.0
    }

    /// The SSL session has been closed.
    pub const ZERO_RETURN: ErrorCode = ErrorCode(ffi::SSL_ERROR_ZERO_RETURN);

    /// An attempt to read data from the underlying socket returned `WouldBlock`.
    ///
    /// Wait for read readiness and retry the operation.
    pub const WANT_READ: ErrorCode = ErrorCode(ffi::SSL_ERROR_WANT_READ);

    /// An attempt to write data to the underlying socket returned `WouldBlock`.
    ///
    /// Wait for write readiness and retry the operation.
    pub const WANT_WRITE: ErrorCode = ErrorCode(ffi::SSL_ERROR_WANT_WRITE);

    /// A non-recoverable IO error occurred.
    pub const SYSCALL: ErrorCode = ErrorCode(ffi::SSL_ERROR_SYSCALL);

    /// An error occurred in the SSL library.
    pub const SSL: ErrorCode = ErrorCode(ffi::SSL_ERROR_SSL);

    /// The client hello callback indicated that it needed to be retried.
    ///
    /// Requires OpenSSL 1.1.1 or newer.
    #[cfg(ossl111)]
    pub const WANT_CLIENT_HELLO_CB: ErrorCode = ErrorCode(ffi::SSL_ERROR_WANT_CLIENT_HELLO_CB);
}

#[derive(Debug)]
pub(crate) enum InnerError {
    Io(io::Error),
    Ssl(ErrorStack),
}

/// An SSL error.
#[derive(Debug)]
pub struct Error {
    pub(crate) code: ErrorCode,
    pub(crate) cause: Option<InnerError>,
}

impl Error {
    pub fn code(&self) -> ErrorCode {
        self.code
    }

    pub fn io_error(&self) -> Option<&io::Error> {
        match self.cause {
            Some(InnerError::Io(ref e)) => Some(e),
            _ => None,
        }
    }

    pub fn into_io_error(self) -> Result<io::Error, Error> {
        match self.cause {
            Some(InnerError::Io(e)) => Ok(e),
            _ => Err(self),
        }
    }

    pub fn ssl_error(&self) -> Option<&ErrorStack> {
        match self.cause {
            Some(InnerError::Ssl(ref e)) => Some(e),
            _ => None,
        }
    }
}

impl From<ErrorStack> for Error {
    fn from(e: ErrorStack) -> Error {
        Error {
            code: ErrorCode::SSL,
            cause: Some(InnerError::Ssl(e)),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self.code {
            ErrorCode::ZERO_RETURN => fmt.write_str("the SSL session has been shut down"),
            ErrorCode::WANT_READ => match self.io_error() {
                Some(_) => fmt.write_str("a nonblocking read call would have blocked"),
                None => fmt.write_str("the operation should be retried"),
            },
            ErrorCode::WANT_WRITE => match self.io_error() {
                Some(_) => fmt.write_str("a nonblocking write call would have blocked"),
                None => fmt.write_str("the operation should be retried"),
            },
            ErrorCode::SYSCALL => match self.io_error() {
                Some(err) => write!(fmt, "{}", err),
                None => fmt.write_str("unexpected EOF"),
            },
            ErrorCode::SSL => match self.ssl_error() {
                Some(e) => write!(fmt, "{}", e),
                None => fmt.write_str("OpenSSL error"),
            },
            ErrorCode(code) => write!(fmt, "unknown error code {}", code),
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        "an OpenSSL error"
    }

    fn cause(&self) -> Option<&error::Error> {
        match self.cause {
            Some(InnerError::Io(ref e)) => Some(e),
            Some(InnerError::Ssl(ref e)) => Some(e),
            None => None,
        }
    }
}

/// An error or intermediate state after a TLS handshake attempt.
// FIXME overhaul
#[derive(Debug)]
pub enum HandshakeError<S> {
    /// Setup failed.
    SetupFailure(ErrorStack),
    /// The handshake failed.
    Failure(MidHandshakeSslStream<S>),
    /// The handshake encountered a `WouldBlock` error midway through.
    ///
    /// This error will never be returned for blocking streams.
    WouldBlock(MidHandshakeSslStream<S>),
}

impl<S: fmt::Debug> StdError for HandshakeError<S> {
    fn description(&self) -> &str {
        match *self {
            HandshakeError::SetupFailure(_) => "stream setup failed",
            HandshakeError::Failure(_) => "the handshake failed",
            HandshakeError::WouldBlock(_) => "the handshake was interrupted",
        }
    }

    fn cause(&self) -> Option<&StdError> {
        match *self {
            HandshakeError::SetupFailure(ref e) => Some(e),
            HandshakeError::Failure(ref s) | HandshakeError::WouldBlock(ref s) => Some(s.error()),
        }
    }
}

impl<S: fmt::Debug> fmt::Display for HandshakeError<S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(StdError::description(self))?;
        match *self {
            HandshakeError::SetupFailure(ref e) => write!(f, ": {}", e)?,
            HandshakeError::Failure(ref s) | HandshakeError::WouldBlock(ref s) => {
                write!(f, ": {}", s.error())?;
                let verify = s.ssl().verify_result();
                if verify != X509VerifyResult::OK {
                    write!(f, ": {}", verify)?;
                }
            }
        }
        Ok(())
    }
}

impl<S> From<ErrorStack> for HandshakeError<S> {
    fn from(e: ErrorStack) -> HandshakeError<S> {
        HandshakeError::SetupFailure(e)
    }
}
