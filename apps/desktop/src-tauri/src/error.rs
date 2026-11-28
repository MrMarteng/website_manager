//! Errors as the window receives them: a machine-readable code plus details.
//!
//! Internally errors are strings; those starting with `code:<name> <detail>` carry a code the
//! window translates (see `ui/src/i18n`). Everything else is shown as is.

use serde::Serialize;

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    /// For example `no-credentials`; `None` for plain messages.
    pub code: Option<String>,
    pub detail: String,
}

impl From<String> for CommandError {
    fn from(message: String) -> Self {
        match message.strip_prefix("code:") {
            Some(rest) => {
                let (code, detail) = rest.split_once(' ').unwrap_or((rest, ""));
                Self {
                    code: Some(code.to_owned()),
                    detail: detail.trim().to_owned(),
                }
            }
            None => Self {
                code: None,
                detail: message,
            },
        }
    }
}

impl From<&str> for CommandError {
    fn from(message: &str) -> Self {
        Self::from(message.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_split_from_details() {
        assert_eq!(
            CommandError::from("code:newer-on-server 1.2.0 2026-11-27T08:00:00Z"),
            CommandError {
                code: Some("newer-on-server".into()),
                detail: "1.2.0 2026-11-27T08:00:00Z".into()
            }
        );
        assert_eq!(
            CommandError::from("code:no-credentials"),
            CommandError {
                code: Some("no-credentials".into()),
                detail: String::new()
            }
        );
        assert_eq!(CommandError::from("disk full").code, None);
    }
}
