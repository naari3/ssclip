use std::result;

use confy::ConfyError;
use thiserror::Error;

pub type Result<T> = result::Result<T, SSClipError>;

#[derive(Debug, Error)]
pub enum SSClipError {
    #[error("Failed to load config: {0:?}")]
    ConfigError(#[from] ConfyError),
    #[error("Failed to copy image: {0:?}")]
    CopyError(#[from] arboard::Error),
    #[error("notify error: {0:?}")]
    Notify(#[from] notify::Error),
    #[error("trayicon error: {0:?}")]
    Trayicon(#[from] trayicon::Error),
    // #[error(transparent)]
    // CrossBeamSendError(#[from] crossbeam_channel::SendError),
    #[error(transparent)]
    CrossBeamRecvError(#[from] crossbeam_channel::RecvError),
}
