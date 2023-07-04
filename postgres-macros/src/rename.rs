use std::str::FromStr;
use std::{error, fmt};

use heck::{
    ToKebabCase, ToLowerCamelCase, ToShoutyKebabCase, ToShoutySnakeCase, ToSnakeCase, ToTrainCase,
    ToUpperCamelCase,
};

#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
pub enum RenameAll {
    CamelCase,
    KebabCase,
    LowerCase,
    PascalCase,
    ScreamingKebabCase,
    ScreamingSnakeCase,
    SnakeCase,
    TrainCase,
    UpperCase,
}

impl RenameAll {
    pub fn apply(&self, name: &str) -> String {
        match self {
            RenameAll::CamelCase => name.to_lower_camel_case(),
            RenameAll::KebabCase => name.to_kebab_case(),
            RenameAll::LowerCase => name.to_lowercase(),
            RenameAll::PascalCase => name.to_upper_camel_case(),
            RenameAll::ScreamingKebabCase => name.to_shouty_kebab_case(),
            RenameAll::ScreamingSnakeCase => name.to_shouty_snake_case(),
            RenameAll::SnakeCase => name.to_snake_case(),
            RenameAll::TrainCase => name.to_train_case(),
            RenameAll::UpperCase => name.to_uppercase(),
        }
    }
}

impl FromStr for RenameAll {
    type Err = InvalidCase;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "camelCase" => Ok(RenameAll::CamelCase),
            "kebab-case" => Ok(RenameAll::KebabCase),
            "lowercase" => Ok(RenameAll::LowerCase),
            "PascalCase" => Ok(RenameAll::PascalCase),
            "SCREAMING-KEBAB-CASE" => Ok(RenameAll::ScreamingKebabCase),
            "SCREAMING_SNAKE_CASE" => Ok(RenameAll::ScreamingSnakeCase),
            "snake_case" => Ok(RenameAll::SnakeCase),
            "Train-Case" => Ok(RenameAll::TrainCase),
            "UPPERCASE" => Ok(RenameAll::UpperCase),
            _ => Err(InvalidCase(())),
        }
    }
}

#[derive(Debug)]
pub struct InvalidCase(());

impl error::Error for InvalidCase {}

impl fmt::Display for InvalidCase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid case conversion")
    }
}
