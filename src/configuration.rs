use config::Config;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Settings {
    pub application_settings: ApplicationSetting,
    pub database_settings: DatabaseSettings,
}

#[derive(Deserialize)]
pub struct ApplicationSetting {
    pub port: u16,
    pub host: String,
}

#[derive(Deserialize)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: SecretString,
    pub port: u16,
    pub host: String,
    pub database_name: String,
}

impl DatabaseSettings {
    pub fn connection_string(&self) -> SecretString {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port,
            self.database_name
        )
        .into()
    }

    pub fn connection_string_without_db(&self) -> SecretString {
        format!(
            "postgres://{}:{}@{}:{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port
        )
        .into()
    }
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let base_path = std::env::current_dir().expect("Couldn't get the current directory");
    let config_dir = base_path.join("configuration");
    let environment: AppEnvironment = std::env::var("APP_ENVIRONMENT")
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .expect("Failed tp parse APP_ENVIRONMENT");
    let environment_filename = format!("{}.yaml", environment.as_str());

    let settings = Config::builder()
        .add_source(config::File::from(config_dir.join("base.yaml")))
        .add_source(config::File::from(config_dir.join(environment_filename)))
        .build()?;
    settings.try_deserialize()
}

pub enum AppEnvironment {
    Local,
    Production,
}

impl AppEnvironment {
    pub fn as_str(&self) -> &'static str {
        match self {
            AppEnvironment::Local => "local",
            AppEnvironment::Production => "production",
        }
    }
}

impl TryFrom<String> for AppEnvironment {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "local" => Ok(AppEnvironment::Local),
            "production" => Ok(AppEnvironment::Production),
            other => Err(format!(
                "{} is not a supported app environment. Use either `local` or `production`",
                other
            )),
        }
    }
}
