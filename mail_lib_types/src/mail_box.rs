/*!
#  MailBox

A [MailBox] is a structure that contains a [EmailAddress] and an optional name.

Defined in [RFC 5322 Section 3.4](https://tools.ietf.org/html/rfc5322#section-3.4)
 */
use std::{fmt::Display, str::FromStr};

use chumsky::{error::Cheap, Parser};
use digestible::Digestible;
use thiserror::Error;

use crate::{parsers::rfcs::rfc5322::mailbox, EmailAddress};
/// Used Internally as a temporary structure to build a [MailBox]
#[doc(hidden)]
#[derive(Debug, PartialEq, Eq)]
pub struct RawMailBox<'a> {
    pub(crate) display_name: Option<&'a str>,
    pub(crate) local: &'a str,
    pub(crate) domain: &'a str,
}
impl RawMailBox<'_> {
    #[inline(always)]
    pub(crate) fn new<'a>(
        display_name: Option<&'a str>,
        local: &'a str,
        domain: &'a str,
    ) -> RawMailBox<'a> {
        RawMailBox {
            display_name,
            local,
            domain,
        }
    }
    #[inline(always)]
    pub(crate) fn new_no_name<'a>(local: &'a str, domain: &'a str) -> RawMailBox<'a> {
        RawMailBox {
            display_name: None,
            local,
            domain,
        }
    }
}
impl<'a> TryFrom<&'a str> for RawMailBox<'a> {
    type Error = InvalidMailBox;
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let parsed = mailbox().parse(value).into_result();
        match parsed {
            Ok(v) => Ok(v.into()),
            Err(e) => Err(InvalidMailBox {
                spans: e,
                ctx: Some(value.to_owned()),
            }),
        }
    }
}

impl Into<(Option<String>, String, String)> for RawMailBox<'_> {
    fn into(self) -> (Option<String>, String, String) {
        (
            self.display_name.map(|v| v.to_owned()),
            self.local.to_owned(),
            self.domain.to_owned(),
        )
    }
}
/// A [MailBox] is a structure that contains a [EmailAddress] and an optional name.
#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Digestible)]
pub struct MailBox {
    /// The optional name of the mailbox
    pub name: Option<String>,
    /// The email address of the mailbox
    pub email: EmailAddress,
}
impl Display for MailBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.name.as_deref() {
            Some(name) => {
                if name.contains(' ') {
                    write!(f, "\"{}\"", name)
                } else {
                    write!(f, "{}", name)
                }
            }
            None => write!(f, "{}", self.email),
        }
    }
}
impl MailBox {
    /// Create a new [MailBox] with the given name and email address
    pub fn new(name: Option<String>, email: EmailAddress) -> Self {
        Self { name, email }
    }
    /// Get the local part of the email address
    pub fn get_local(&self) -> &str {
        self.email.get_local()
    }
    /// Get the domain part of the email address
    pub fn get_domain(&self) -> &str {
        self.email.get_domain()
    }
    /// Get the name of the mailbox
    pub fn get_name(&self) -> Option<&str> {
        self.name.as_deref()
    }
    /// Convert the [MailBox] into its parts
    pub fn into_inner(self) -> (Option<String>, EmailAddress) {
        (self.name, self.email)
    }
}
#[cfg(feature = "serde")]
mod _serde {
    use serde::{ser::SerializeStruct, Deserialize, Serialize};

    use super::MailBox;
    use crate::EmailAddress;
    /// Serialize a [MailBox] as an object with the fields `name` and `email`
    pub fn serialize_as_object<S>(mailbox: &MailBox, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut ser = serializer.serialize_struct("MailBox", 2)?;
        ser.serialize_field("name", &mailbox.name)?;
        ser.serialize_field("email", &mailbox.email)?;
        ser.end()
    }

    impl Serialize for MailBox {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            match &self.name {
                Some(name) => serializer.serialize_str(&format!("{} <{}>", name, self.email)),
                None => serializer.serialize_str(self.email.as_ref()),
            }
        }
    }
    struct MailBoxVisitor;

    impl<'de> serde::de::Visitor<'de> for MailBoxVisitor {
        type Value = MailBox;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid email address")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            MailBox::try_from(value).map_err(serde::de::Error::custom)
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            MailBox::try_from(v).map_err(serde::de::Error::custom)
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            let mut name: Option<String> = None;
            let mut email: Option<String> = None;
            while let Some(key) = map.next_key::<String>()? {
                match key.as_str() {
                    "name" => {
                        if name.is_some() {
                            return Err(serde::de::Error::duplicate_field("name"));
                        }
                        name = Some(map.next_value()?);
                    }
                    "email" => {
                        if email.is_some() {
                            return Err(serde::de::Error::duplicate_field("email"));
                        }
                        email = Some(map.next_value()?);
                    }
                    _ => {
                        return Err(serde::de::Error::unknown_field(
                            key.as_str(),
                            &["name", "email"],
                        ))
                    }
                }
            }
            let email = email.ok_or_else(|| serde::de::Error::missing_field("email"))?;
            let address = EmailAddress::new(email.as_str()).map_err(serde::de::Error::custom)?;
            Ok(MailBox::new(name, address))
        }
    }
    impl<'de> Deserialize<'de> for MailBox {
        fn deserialize<D>(deserializer: D) -> Result<MailBox, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_any(MailBoxVisitor)
        }
    }
}
#[cfg(feature = "serde")]
#[doc(inline)]
pub use _serde::serialize_as_object;
/// An error that occurs when parsing a [MailBox]
#[derive(Debug, Clone, PartialEq, Hash, Error)]
pub struct InvalidMailBox {
    /// The spans that caused the error
    pub spans: Vec<Cheap>,
    /// The context of the error
    pub ctx: Option<String>,
}
impl Display for InvalidMailBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Invalid MailBox")?;
        if let Some(context) = self.ctx.as_ref() {
            writeln!(f, "Context: {}", context)?;
            for span in &self.spans {
                writeln!(f, "    {}", span)?;
            }
        }
        Ok(())
    }
}
impl From<RawMailBox<'_>> for MailBox {
    #[inline(always)]
    fn from(value: RawMailBox) -> Self {
        let RawMailBox {
            display_name,
            local,
            domain,
        } = value;
        // Safe as long as the parser did its job
        let email = unsafe { EmailAddress::new_unchecked_from_parts(local, domain) };
        MailBox::new(display_name.map(|v| v.to_owned()), email)
    }
}
impl PartialEq<RawMailBox<'_>> for MailBox {
    fn eq(&self, other: &RawMailBox) -> bool {
        other.display_name == self.name.as_deref()
            && self.email.get_local() == other.local
            && self.email.get_domain() == other.domain
    }
}
impl FromStr for MailBox {
    type Err = InvalidMailBox;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        RawMailBox::try_from(value).map(MailBox::from)
    }
}
impl TryFrom<String> for MailBox {
    type Error = InvalidMailBox;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        RawMailBox::try_from(value.as_str()).map(MailBox::from)
    }
}
impl<'a> TryFrom<&'a String> for MailBox {
    type Error = InvalidMailBox;
    fn try_from(value: &'a String) -> Result<Self, Self::Error> {
        RawMailBox::try_from(value.as_str()).map(MailBox::from)
    }
}
impl<'a> TryFrom<&'a str> for MailBox {
    type Error = InvalidMailBox;
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        RawMailBox::try_from(value).map(MailBox::from)
    }
}
