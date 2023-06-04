use x11rb::{
    rust_connection::{ConnectError, ConnectionError, ParseError, ReplyError, ReplyOrIdError},
    x11_utils::X11Error,
};

#[derive(Debug)]
pub enum RbError {
    ConnectError(ConnectError),
    ConnectionError(ConnectionError),
    ParseError(ParseError),
    ReplyError(ReplyError),
    IdsExausted(),
    X11Error(X11Error),
}

impl From<ConnectError> for RbError {
    fn from(value: ConnectError) -> Self {
        Self::ConnectError(value)
    }
}

impl From<ConnectionError> for RbError {
    fn from(value: ConnectionError) -> Self {
        Self::ConnectionError(value)
    }
}

impl From<ParseError> for RbError {
    fn from(value: ParseError) -> Self {
        Self::ParseError(value)
    }
}

impl From<ReplyError> for RbError {
    fn from(value: ReplyError) -> Self {
        Self::ReplyError(value)
    }
}

impl From<X11Error> for RbError {
    fn from(value: X11Error) -> Self {
        Self::X11Error(value)
    }
}

impl From<ReplyOrIdError> for RbError {
    fn from(value: ReplyOrIdError) -> Self {
        match value {
            ReplyOrIdError::IdsExhausted => Self::IdsExausted(),
            ReplyOrIdError::ConnectionError(e) => e.into(),
            ReplyOrIdError::X11Error(e) => e.into(),
        }
    }
}
