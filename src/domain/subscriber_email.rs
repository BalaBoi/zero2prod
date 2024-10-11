use validator::validate_email;

#[derive(Debug)]
pub struct SubscriberEmail(String);

impl SubscriberEmail {
    pub fn parse(s: &str) -> Result<Self, String> {
        if validate_email(s) {
            Ok(Self(s.into()))
        } else {
            Err("not a valid email".into())
        }
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use claim::assert_err;
    use crate::domain::SubscriberEmail;

    #[test]
    fn empty_email_is_rejected() {
        let s = "";
        assert_err!(SubscriberEmail::parse(s));
    }

    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let s = "someone.com";
        assert_err!(SubscriberEmail::parse(s));
    }

    #[test]
    fn email_missing_subject_is_rejected() {
        let s = "@domain.com";
        assert_err!(SubscriberEmail::parse(s));
    }
}