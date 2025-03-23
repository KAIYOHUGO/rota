use std::{convert::Infallible, str::FromStr};

use knus::{Decode, DecodeScalar};

#[derive(Debug, Decode)]
pub struct Config {
    #[knus(child)]
    pub settings: Settings,
    #[knus(children(name = "varibles"))]
    pub varibles: Vec<Variables>,
    #[knus(children(name = "actions"))]
    pub actions: Vec<Actions>,
}

#[derive(Debug, Decode)]
pub struct Settings {
    #[knus(child, unwrap(argument))]
    pub default_mode: SettingMode,
    #[knus(child, unwrap(argument))]
    pub switch: String,
    #[knus(child, unwrap(argument))]
    pub touchscreen: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, DecodeScalar)]
pub enum SettingMode {
    Laptop,
    Tablet,
}

#[derive(Debug, Decode)]
pub struct Variables {
    #[knus(children)]
    pub variables: Vec<Variable>,
}

#[derive(Debug, Decode)]
pub struct Variable {
    #[knus(node_name)]
    pub name: String,
    #[knus(argument, str)]
    pub value: VStr,
}

#[derive(Debug, Decode)]
pub struct Actions {
    #[knus(children)]
    pub actions: Vec<Action>,
}

#[derive(Debug, Decode)]
pub struct Action {
    #[knus(node_name)]
    pub event: String,
    #[knus(children)]
    pub tasks: Vec<Task>,
}

#[derive(Debug, Decode)]
pub enum Task {
    Action(#[knus(argument, str)] VStr),

    Cmd(#[knus(arguments, str)] Vec<VStr>),

    Rotation(#[knus(argument, str)] VStr),

    RotateCalibration(#[knus(argument, str)] VStr),
}

#[derive(Debug)]
pub enum VStr {
    Value(String),
    Config(String),
}

impl FromStr for VStr {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with("@") {
            return Ok(Self::Config(s[1..].to_owned()));
        }
        return Ok(Self::Value(s.to_owned()));
    }
}
