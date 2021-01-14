use std::fmt;

use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

use crate::api::models::contract::ContractKind;

#[derive(Debug, Deserialize, Clone)]
pub struct Server {
    pub address: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Database {
    pub host: String,
    pub port: String,
    pub user: String,
    pub password: String,
    pub name: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SMTP {
    pub host: String,
    pub port: String,
    pub user: String,
    pub password: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Tezos {
    pub node_url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Contract {
    pub address: String,
    pub multisig: String,
    pub name: String,
    pub kind: ContractKind,
    pub token_id: i64,
    pub gatekeepers: Vec<Gatekeeper>,
    pub keyholders: Vec<Keyholder>,
    pub decimals: i32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Gatekeeper {
    pub public_key: String,
    pub name: String,
    pub email: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Keyholder {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub server: Server,
    pub database: Database,
    pub smtp: SMTP,
    pub tezos: Tezos,
    pub contracts: Vec<Contract>,
    pub env: ENV,
}

#[derive(Clone, Debug, Deserialize)]
pub enum ENV {
    Development,
    Testing,
    Production,
    Local,
}

impl fmt::Display for ENV {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ENV::Development => write!(f, "Development"),
            ENV::Testing => write!(f, "Testing"),
            ENV::Production => write!(f, "Production"),
            ENV::Local => write!(f, "Local"),
        }
    }
}

impl From<&str> for ENV {
    fn from(env: &str) -> Self {
        match env {
            "Testing" => ENV::Testing,
            "Production" => ENV::Production,
            "Development" => ENV::Development,
            _ => ENV::Local,
        }
    }
}

const CONFIG_FILE_PATH: &str = "./config/Default.toml";
const CONFIG_FILE_PREFIX: &str = "./config/";

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let env = std::env::var("RUN_ENV").unwrap_or_else(|_| "Local".into());
        let mut s = Config::new();
        s.set("env", env.clone())?;
        println!("RUN ENV: {}", env);
        s.merge(File::with_name(CONFIG_FILE_PATH))?;
        s.merge(File::with_name(&format!("{}{}", CONFIG_FILE_PREFIX, env)))?;

        // This makes it so "TZW_SERVER__ADDRESS overrides server.address
        s.merge(Environment::with_prefix("tzw").separator("__"))?;

        s.try_into()
    }
}
