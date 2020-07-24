use structopt::StructOpt;

#[derive(Debug)]
pub struct PresetDoesNotExist;

impl std::fmt::Display for PresetDoesNotExist {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Requested preset does not exist.")
    }
}

#[derive(Debug)]
pub enum ParseDefError {
    Int(std::num::ParseIntError),
    Preset(PresetDoesNotExist),
    TooShort,
    TooMany,
}

impl From<std::num::ParseIntError> for ParseDefError {
    fn from(e: std::num::ParseIntError) -> Self {
        Self::Int(e)
    }
}

impl From<PresetDoesNotExist> for ParseDefError {
    fn from(e: PresetDoesNotExist) -> Self {
        Self::Preset(e)
    }
}

impl std::fmt::Display for ParseDefError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Int(e) => e.fmt(f),
            Self::Preset(p) => p.fmt(f),
            Self::TooShort => write!(f, "Not enough arguments provided."),
            Self::TooMany => write!(f, "Too many arguments provided."),
        }
    }
}

#[derive(Debug)]
pub enum Preset {
    Beginner,
    Intermediate,
    Advanced,
}

impl std::fmt::Display for Preset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::Beginner => "Beginner",
            Self::Intermediate => "Intermediate",
            Self::Advanced => "Advanced",
        };
        write!(f, "{}", text)?;
        Ok(())
    }
}

impl std::str::FromStr for Preset {
    type Err = PresetDoesNotExist;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "beginner" => Ok(Self::Beginner),
            "intermediate" => Ok(Self::Intermediate),
            "advanced" => Ok(Self::Advanced),
            _ => Err(PresetDoesNotExist),
        }
    }
}

#[derive(Debug)]
pub enum Def {
    Preset(Preset),
    Descrip {
        width: usize,
        height: Option<usize>,
        mines: u64,
    }
}

impl std::str::FromStr for Def {
    type Err = ParseDefError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<_> = s.split(",").collect();
        match parts.as_slice() {
            [] => Err(Self::Err::TooShort),
            [preset] => Ok(Self::Preset(preset.parse()?)),
            [dim, mines] => Ok(Self::Descrip {
                width: dim.parse()?,
                height: None,
                mines: mines.parse()?,
            }),
            [width, height, mines] => Ok(Self::Descrip {
                width: width.parse()?,
                height: Some(height.parse()?),
                mines: mines.parse()?,
            }),
            _ => Err(Self::Err::TooMany),
        }
    }
}

impl std::fmt::Display for Def {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Preset(p) => {
                write!(f, "Minesweeper preset {}", p)
            }
            Self::Descrip { width, height: Some(height), mines } => {
                write!(f, "Minesweeper {}x{} with {} mines", width, height, mines)
            }
            Self::Descrip { width, height: None, mines } => {
                write!(f, "Minesweeper {}x{} with {} mines", width, width, mines)
            }
        }
    }
}

#[derive(Debug)]
#[derive(StructOpt)]
pub struct Opts {
    #[structopt(default_value = "Def::Preset(Preset::Beginner)")]
    pub def: Def,
}
