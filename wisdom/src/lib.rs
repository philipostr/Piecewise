use std::fmt::Display;

mod dynamic_string;
pub(crate) use dynamic_string::*;
mod event;
pub(crate) use event::*;
mod pieces;
pub(crate) use pieces::*;
mod state;
pub(crate) use state::*;

/* GameReadError */

#[derive(Debug)]
pub enum GameReadError {
    Io(std::io::Error),
    SerdeSaphyr(serde_saphyr::Error),
    DynamicString(String),
    CustomPiece(String),
    State(String),
}

impl From<std::io::Error> for GameReadError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_saphyr::Error> for GameReadError {
    fn from(value: serde_saphyr::Error) -> Self {
        Self::SerdeSaphyr(value)
    }
}

impl Display for GameReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => e.fmt(f),
            Self::SerdeSaphyr(e) => e.fmt(f),
            Self::DynamicString(e) => e.fmt(f),
            Self::CustomPiece(e) => e.fmt(f),
            Self::State(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for GameReadError {}

impl GameReadError {
    pub fn msg(&self) -> String {
        match self {
            GameReadError::Io(error) => error.to_string(),
            GameReadError::SerdeSaphyr(error) => error.to_string(),
            GameReadError::DynamicString(error) => error.clone(),
            GameReadError::CustomPiece(error) => error.clone(),
            GameReadError::State(error) => error.clone(),
        }
    }
}

/* End of GameReadError */

/* GameBuildError */

#[derive(Debug)]
pub enum GameBuildError {
    GameRead(GameReadError),
    Io(std::io::Error),
}

impl From<GameReadError> for GameBuildError {
    fn from(value: GameReadError) -> Self {
        Self::GameRead(value)
    }
}

impl From<std::io::Error> for GameBuildError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl Display for GameBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GameRead(e) => e.fmt(f),
            Self::Io(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for GameBuildError {}

/* End of GameBuildError */

pub fn build(project_path: &str) -> Result<(), GameBuildError> {
    init_custom_piece_types(&format!("{project_path}{}custom_piece_types", std::path::MAIN_SEPARATOR))?;
    let game = Game::from_path(&format!("{project_path}{}wisdom.yaml", std::path::MAIN_SEPARATOR))?;

    let target_dirpath = "dist";
    clean_targetdir(target_dirpath)?;
    std::fs::create_dir_all(target_dirpath)?;
    dircpy::copy_dir("wisdom-js", &format!("{target_dirpath}{}wisdom-js", std::path::MAIN_SEPARATOR))?;
    dircpy::copy_dir("public", target_dirpath)?;
    game.build(target_dirpath)?;

    Ok(())
}

fn clean_targetdir(path: &str) -> Result<(), std::io::Error> {
    let path = std::path::Path::new(path);

    if path.exists() {
        if !path.is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotADirectory, 
                "dist exists but is not a directory"
            ))
        } else {
            std::fs::remove_dir_all(path)?;
        }
    }

    Ok(())
}
