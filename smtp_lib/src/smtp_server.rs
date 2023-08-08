use crate::SMTPConnectionState;
use common::credentials::LoginMechanism;
use enum_helper::EnumOfKeys;
use std::fmt::{Debug, Display};
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ServerExtensionParseError {
    #[error("Invalid size value: {0}")]
    InvalidSize(String),
    #[error("Invalid extension: {0}")]
    InvalidExtension(String),
}
#[derive(Debug, Clone, PartialEq, Eq, EnumOfKeys)]
#[enum_of_keys(SMTPServerExtensionKeys)]
#[enum_attr(derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    strum::EnumIs,
    strum::EnumString,
    strum::Display,
    strum::AsRefStr,
    strum::IntoStaticStr
))]
#[enum_attr(strum(serialize_all = "UPPERCASE"))]
pub enum SMTPServerExtension {
    Size(u64),
    StartTLS,
    Auth(Vec<LoginMechanism>),
    #[enum_of_keys(default=name)]
    #[enum_attr(strum(default))]
    Other{
        name: String,
        value: Option<String>,
    }
}
impl Display for SMTPServerExtension {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SMTPServerExtension::Size(size) => write!(f, "SIZE {}", size),
            SMTPServerExtension::StartTLS => write!(f, "STARTTLS"),
            SMTPServerExtension::Auth(value) => {
                write!(f, "AUTH {}", LoginMechanism::format_iter(value.iter()))
            }
            SMTPServerExtension::Other{
                name,
                value,
            } =>{
                if let Some(value) = value {
                    write!(f, "{} {}", name, value)
                } else {
                    write!(f, "{}", name)
                }
            }
        }
    }
}
impl TryFrom<String> for SMTPServerExtension {
    type Error = ServerExtensionParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let mut split = value.split(" ");
        match split
            .next()
            .ok_or(ServerExtensionParseError::InvalidSize(value.clone()))?
        {
            "SIZE" => {
                let size = u64::from_str(
                    split
                        .next()
                        .ok_or(ServerExtensionParseError::InvalidSize(value.clone()))?,
                )
                .map_err(|_| ServerExtensionParseError::InvalidSize(value.clone()))?;
                Ok(Self::Size(size))
            }
            "STARTLS" => Ok(Self::StartTLS),
            "AUTH" => Ok(Self::Auth(LoginMechanism::from_iter(split))),
            other_key => {
                let other_key = other_key.to_string();
                let other_data = value.splitn(2, " ").nth(1).map(|s| s.to_string());
                Ok(Self::Other{
                    name: other_key,
                    value: other_data,
                })
            }
        }
    }
}
///
pub trait SMTPServer: Debug {
    fn get_hostname(&self) -> &str;

    fn name(&self) -> &str;

    fn get_greeting(&self) -> Option<&str>;

    fn supported_extensions(&self) -> &Vec<SMTPServerExtension>;
}

pub trait SMTPConnection: Debug {
    type Server: SMTPServer;

    fn get_server(&self) -> &Self::Server;

    fn get_state(&self) -> &SMTPConnectionState;

    fn set_state(&mut self, state: SMTPConnectionState);

    fn get_end_of_multiline_command(&self) -> &str;
}

pub mod async_traits {
    use crate::error::SMTPError;
    use crate::smtp_server::SMTPConnection;
    use crate::statement::Statement;
    use bytes::Bytes;
    use std::future::Future;

    /// An async version of SMTPConnection
    ///
    /// ### Notes
    /// The Future Types will be dropped when Rust 1.74 goes into beta https://blog.rust-lang.org/inside-rust/2023/05/03/stabilizing-async-fn-in-trait.html#timeline-and-roadmap
    pub trait AsyncSMTPConnection<'a>: SMTPConnection + Send {
        type ReadLineFuture: Future<Output = Result<String, SMTPError>> + Send + 'a;
        type WriteFuture: Future<Output = Result<(), SMTPError>> + Send + 'a;
        type ReadTilEndFuture: Future<Output = Result<String, SMTPError>> + Send + 'a;

        /// Reads the next line from the SMTP Server
        fn read_line(&'a mut self) -> Self::ReadLineFuture;

        fn write(&'a mut self, command: Bytes) -> Self::WriteFuture;
        fn write_statement(&'a mut self, statement: impl Statement) -> Self::WriteFuture {
            self.write(statement.to_bytes())
        }

        /// Reads til
        fn read_til_end(&'a mut self) -> Self::ReadTilEndFuture;
    }
}