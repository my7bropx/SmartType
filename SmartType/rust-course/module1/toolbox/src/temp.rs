use std::error::Error;
use std::fmt;
use std::num::ParseFloatError;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Scale {
    Celsius,
    Fahrenheit,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Temperature {
    pub value: f64,
    pub scale: Scale,
}

impl Temperature {
    pub fn parse(input: &str) -> Result<Self, TempParseError> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(TempParseError::MissingInput);
        }

        let (number_part, unit_part) = trimmed.split_at(trimmed.len() - 1);
        let value = number_part
            .trim()
            .parse::<f64>()
            .map_err(TempParseError::Value)?;

        let scale = match unit_part.to_ascii_lowercase().as_str() {
            "c" => Scale::Celsius,
            "f" => Scale::Fahrenheit,
            _ => return Err(TempParseError::UnknownUnit(unit_part.to_string())),
        };

        Ok(Self { value, scale })
    }

    pub fn to_celsius(self) -> Self {
        match self.scale {
            Scale::Celsius => self,
            Scale::Fahrenheit => Self {
                value: (self.value - 32.0) * 5.0 / 9.0,
                scale: Scale::Celsius,
            },
        }
    }

    pub fn to_fahrenheit(self) -> Self {
        match self.scale {
            Scale::Fahrenheit => self,
            Scale::Celsius => Self {
                value: (self.value * 9.0 / 5.0) + 32.0,
                scale: Scale::Fahrenheit,
            },
        }
    }
}

impl fmt::Display for Temperature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let unit = match self.scale {
            Scale::Celsius => "°C",
            Scale::Fahrenheit => "°F",
        };
        write!(f, "{:.2}{}", self.value, unit)
    }
}

#[derive(Debug, PartialEq)]
pub enum TempParseError {
    MissingInput,
    Value(ParseFloatError),
    UnknownUnit(String),
}

impl fmt::Display for TempParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TempParseError::MissingInput => write!(f, "no input provided"),
            TempParseError::Value(e) => write!(f, "invalid numeric value: {e}"),
            TempParseError::UnknownUnit(unit) => write!(f, "unknown temperature unit: {unit}"),
        }
    }
}

impl Error for TempParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            TempParseError::Value(e) => Some(e),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_temperature() {
        assert_eq!(
            Temperature::parse("30C").unwrap(),
            Temperature {
                value: 30.0,
                scale: Scale::Celsius
            }
        );
        assert_eq!(Temperature::parse("86f").unwrap().scale, Scale::Fahrenheit);
        assert!(Temperature::parse("10").is_err());
    }

    #[test]
    fn converts_correctly() {
        let cold = Temperature {
            value: 0.0,
            scale: Scale::Celsius,
        };
        let hot = Temperature {
            value: 212.0,
            scale: Scale::Fahrenheit,
        };

        assert!((cold.to_fahrenheit().value - 32.0).abs() < 1e-6);
        assert!((hot.to_celsius().value - 100.0).abs() < 1e-6);
    }
}
