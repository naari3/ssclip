use serde::{Deserialize, Serialize};

const APP_NAME: &str = env!("CARGO_PKG_NAME");

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Config {
    pub paths: Vec<String>,
}

impl Config {
    pub fn load() -> Result<Self, confy::ConfyError> {
        confy::load::<Config>(APP_NAME, APP_NAME)
    }

    #[allow(dead_code)]
    pub fn save(&self) -> Result<(), confy::ConfyError> {
        confy::store(APP_NAME, APP_NAME, &self)
    }

    #[allow(dead_code)]
    pub fn reload(&mut self) -> Result<(), confy::ConfyError> {
        *self = Self::load()?;
        Ok(())
    }
}
