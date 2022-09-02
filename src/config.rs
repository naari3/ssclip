use notify::{EventHandler, Watcher};
use serde::{Deserialize, Serialize};

const APP_NAME: &str = env!("CARGO_PKG_NAME");

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
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

    pub fn reload(&mut self) -> Result<(), confy::ConfyError> {
        println!("{:?}", Config::load()?);
        *self = Self::load()?;
        Ok(())
    }

    pub fn get_watcher<H: EventHandler>(&self, handler: H) -> notify::RecommendedWatcher {
        let mut watcher = notify::recommended_watcher(handler).unwrap();
        watcher
            .watch(
                confy::get_configuration_file_path(APP_NAME, APP_NAME)
                    .unwrap()
                    .as_ref(),
                notify::RecursiveMode::Recursive,
            )
            .unwrap();
        watcher
    }
}
