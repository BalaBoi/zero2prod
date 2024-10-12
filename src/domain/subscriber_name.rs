use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug)]
pub struct SubscriberName(String);

impl SubscriberName {
    /// Returns an instance of SubscriberName
    /// if the input satisfies all the constraints,
    /// panics otherwise.
    pub fn parse(s: &str) -> Result<Self, String> {
        if s.trim().is_empty() {
            return Err("subscriber name is either empty or just whitespaces".into());
        }

        if s.graphemes(true).count() > 256 {
            return Err("subscriber name is too long".into());
        }

        let forbidden_characters = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];
        if s.chars().any(|char| forbidden_characters.contains(&char)) {
            return Err("subscriber name contains invalid characters".into());
        }

        Ok(Self(s.into()))
    }
}

impl AsRef<str> for SubscriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::SubscriberName;
    use claim::{assert_err, assert_ok};

    #[test]
    fn a_256_grapheme_long_name_is_valid() {
        let s = "ё".repeat(256);
        assert_ok!(SubscriberName::parse(&s));
    }

    #[test]
    fn a_name_longer_than_256_graphemes_is_rejected() {
        let s = "t".repeat(350);
        assert_err!(SubscriberName::parse(&s));
    }

    #[test]
    fn empty_name_is_rejected() {
        let s = "";
        assert_err!(SubscriberName::parse(s));
    }

    #[test]
    fn whitespace_name_is_rejected() {
        let s = "   ";
        assert_err!(SubscriberName::parse(s));
    }

    #[test]
    fn name_with_invalid_characters_is_rejected() {
        let s = "\\ wo (";
        assert_err!(SubscriberName::parse(s));
    }
}
