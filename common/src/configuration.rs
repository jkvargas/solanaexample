use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;
use serde::{Serialize, Deserialize};
use solana_sdk::pubkey::{ParsePubkeyError, Pubkey};
use solana_sdk::signature::Keypair;

#[derive(Debug)]
pub struct Configuration {
    pub key_pair: Keypair,
}

#[derive(Serialize, Deserialize, Default, Debug)]
struct ConfigurationFile {
    pub key_pair: String,
}

const CONFIG_DIR: &'static str = "solclient";

impl Configuration {
    pub fn load() -> Result<Self, Box<dyn Error>> {
        let file_config = ConfigurationFile::load()?;
        let bytes = Self::parse(file_config.key_pair)?;
        let key_pair = Keypair::from_bytes(&bytes)?;

        Ok(Self {
            key_pair,
        })
    }

    fn parse_program_id(program_id: String) -> Pubkey {
        Pubkey::from_str(program_id.as_str()).unwrap()
    }

    fn parse(keypair: String) -> Result<Vec<u8>, Box<dyn Error>> {
        let result: Vec<u8> = serde_json::from_str(keypair.as_str())?;
        Ok(result)
    }
}

impl ConfigurationFile {
    pub fn load() -> Result<Self, Box<dyn Error>> {
        let res: ConfigurationFile = confy::load(CONFIG_DIR)?;
        Ok(res)
    }

    pub fn store(&self) -> Result<(), Box<dyn Error>> {
        confy::store(CONFIG_DIR, self)?;
        Ok(())
    }
}