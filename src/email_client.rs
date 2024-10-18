use crate::domain::SubscriberEmail;
use reqwest::Client;
use secrecy::{ExposeSecret, SecretString};
use serde_json::json;

pub struct EmailClient {
    sender: SubscriberEmail,
    http_client: Client,
    base_url: String,
    authorization_token: SecretString,
}

impl EmailClient {
    pub fn new(
        base_url: &str,
        sender: SubscriberEmail,
        authorization_token: &SecretString,
        timeout: std::time::Duration,
    ) -> Self {
        let client = Client::builder().timeout(timeout).build().unwrap();
        Self {
            http_client: client,
            base_url: base_url.to_owned(),
            sender,
            authorization_token: authorization_token.clone(),
        }
    }

    pub async fn send_email(
        &self,
        recipient: &SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<(), reqwest::Error> {
        let email_api = format!("{}/v3/mail/send", self.base_url);

        self.http_client
            .post(&email_api)
            .header(
                "Authorization",
                format!("Bearer {}", self.authorization_token.expose_secret()),
            )
            .json(&json!({
                "personalizations": [
                    {
                        "to": [
                            {"email": recipient.as_ref()}
                        ]
                    }
                ],
                "from": {
                    "email": self.sender.as_ref()
                },
                "subject": subject,
                "content": [
                    {
                        "type": "text/plain",
                        "value": text_content
                    },
                    {
                        "type": "text/html",
                        "value": html_content
                    }
                ]
            }))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{domain::SubscriberEmail, email_client::EmailClient};
    use claim::{assert_err, assert_ok};
    use fake::{
        faker::{
            internet::en::SafeEmail,
            lorem::en::{Paragraph, Sentence},
        },
        Fake, Faker,
    };
    use secrecy::SecretString;
    use wiremock::{matchers::any, Mock, MockServer, ResponseTemplate};

    fn subject() -> String {
        Sentence(1..2).fake()
    }

    fn content() -> String {
        Paragraph(1..10).fake()
    }

    fn email() -> SubscriberEmail {
        SubscriberEmail::parse(&SafeEmail().fake::<String>()).unwrap()
    }

    fn email_client(base_url: &str) -> EmailClient {
        EmailClient::new(
            base_url,
            email(),
            &SecretString::new(Faker.fake::<String>().into_boxed_str()),
            std::time::Duration::from_millis(200),
        )
    }

    #[tokio::test]
    async fn send_email_fires_a_request_to_base_url() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(&mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let out = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;

        assert_ok!(out);
    }

    #[tokio::test]
    async fn send_email_fails_if_the_server_returns_500() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(&mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        let out = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;

        assert_err!(out);
    }

    #[tokio::test]
    async fn send_email_times_out_if_the_server_takes_too_long() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(&mock_server.uri());

        let response = ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(100));
        Mock::given(any())
            .respond_with(response)
            .expect(1)
            .mount(&mock_server)
            .await;

        let out = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;

        assert_err!(out);
    }
}
