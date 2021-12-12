use common::{
    configuration::Configuration,
    protocol::{SolResult, SolanaService},
};
use log::error;
use std::{error::Error, process::exit};

fn main() {
    env_logger::init();

    fn execute_application() -> Result<(), Box<dyn Error>> {
        let config = Configuration::load()?;
        let mut service = SolanaService::new(config)?;

        service.connect()?;

        if !service.is_program_deployed()? {
            error!("program was not deployed");
            exit(1);
        }

        if !service.is_program_deployed()? {
            error!("program was not deployed");
            exit(1);
        }

        let program_instance_account = service.get_or_create_program_instance_account()?;

        service.add_to_counter(&program_instance_account)?;

        Ok(())
    }

    if let Err(err) = execute_application() {
        error!("{}", err.to_string());
        exit(1);
    }
}
