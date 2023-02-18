use std::path::PathBuf;

use serde::{Deserialize, Serialize};

const APP_NAME: &str = env!("CARGO_PKG_NAME");

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct PathIter {
    paths: Vec<PathBuf>,
}

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

    pub fn get_config_path() -> std::path::PathBuf {
        confy::get_configuration_file_path(APP_NAME, APP_NAME).unwrap()
    }

    pub fn path_iter(&self) -> PathIter {
        PathIter::from_strs(self.paths.clone())
    }
}

impl PathIter {
    pub fn from_strs(paths: Vec<String>) -> Self {
        let mut paths = paths
            .iter()
            .flat_map(|path| glob::glob(path.as_str()).unwrap().filter_map(Result::ok))
            .collect::<Vec<PathBuf>>();
        paths.sort();
        Self { paths }
    }
}

impl Iterator for PathIter {
    type Item = PathBuf;

    fn next(&mut self) -> Option<Self::Item> {
        self.paths.pop()
    }
}
