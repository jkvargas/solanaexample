use crate::configuration::Configuration;
use contract::GreetingAccount;
use solana_client::rpc_client::RpcClient;
use solana_sdk::account::Account;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    info, msg,
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
use std::{error::Error, fmt};
use std::convert::TryInto;
use solana_sdk::instruction::{AccountMeta, Instruction};
use borsh::ser::BorshSerialize;

const SEED: &'static str = "WHATEVER";

pub struct SolanaService {
    json_rpc_url: String,
    payer_keypair: Keypair,
    client: Option<RpcClient>,
    program_keypair: Keypair,
    payer_account: Option<Account>,
}

#[derive(Clone, Copy)]
pub enum ProtocolError {
    ClientNotConnected,
    AccountIsNotExecutable,
    KeyPairForProgramNotAvailable,
    ConfigFilePathNotFound,
    KeyPairForPayerNotFound,
}

impl fmt::Debug for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{ file: {}, line: {} }}", file!(), line!())
    }
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error")
    }
}

impl Error for ProtocolError {}

pub type SolResult<T> = Result<T, Box<dyn Error>>;

impl SolanaService {
    pub fn new(configuration: Configuration) -> SolResult<Self> {
        let config_file = solana_cli_config::CONFIG_FILE
            .as_ref()
            .ok_or_else(|| Box::new(ProtocolError::ConfigFilePathNotFound))?;
        let cli_config = solana_cli_config::Config::load(&config_file)?;
        let json_rpc_url = cli_config.json_rpc_url;
        let local_keypair = read_keypair_file(&cli_config.keypair_path)?;

        Ok(SolanaService {
            json_rpc_url,
            payer_keypair: local_keypair,
            client: None,
            program_keypair: configuration.key_pair,
            payer_account: None,
        })
    }

    pub fn connect(&mut self) -> SolResult<()> {
        msg!("connecting to solana node at {}", self.json_rpc_url);

        let client = RpcClient::new_with_commitment(
            self.json_rpc_url.clone(),
            CommitmentConfig::confirmed(),
        );

        let version = client.get_version()?;
        msg!("RPC version: {:?}", version);

        let account = client.get_account(&self.payer_keypair.pubkey())?;

        msg!("payer account: {:?}", account);

        self.payer_account = Some(account);
        self.client = Some(client);

        Ok(())
    }

    // the one deployed
    pub fn is_program_deployed(&mut self) -> SolResult<bool> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| Box::new(ProtocolError::ClientNotConnected))?;

        let program_pubkey = self.program_keypair.pubkey();

        msg!("program pubkey: {}", program_pubkey);

        let account = client.get_account(&program_pubkey)?;

        msg!("program account: {:?}", account);

        if !account.executable {
            return Ok(false);
        }

        Ok(true)
    }

    pub fn add_to_counter(&self, program_key: &Pubkey) -> SolResult<()> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| Box::new(ProtocolError::ClientNotConnected))?;

        let greetings = GreetingAccount::default();
        let mut data : Vec<u8> = Vec::new();
        greetings.serialize(&mut data).unwrap();

        let instruction = Instruction {
            program_id: self.program_keypair.pubkey(),
            accounts: vec![AccountMeta::new(*program_key, false)],
            data,
        };

        let mut transaction = Transaction::new_with_payer(&[instruction], Some(&self.payer_keypair.pubkey()));

        let block_hash = client.get_recent_blockhash()?.0;

        transaction.try_sign(&[&self.payer_keypair], block_hash)?;

        let sig = client.send_and_confirm_transaction_with_spinner(&transaction)?;

        msg!("claim sig: {}", &sig);

        Ok(())
    }

    pub fn get_or_create_program_instance_account(&self) -> SolResult<Pubkey> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| ProtocolError::ClientNotConnected)?;

        let pubkey = Pubkey::create_with_seed(
            &self.payer_keypair.pubkey(),
            SEED,
            &self.program_keypair.pubkey(),
        )?;

        msg!("program account pubkey: {}", pubkey);

        let account = client.get_account(&pubkey);

        if account.is_err() {
            Self::create_account_instance(
                client,
                &pubkey,
                &self.payer_keypair,
                &self.program_keypair,
            );
        }

        let account = client.get_account(&pubkey)?;

        msg!("program instance account: {:?}", account);

        Ok(pubkey)
    }

    fn create_account_instance(
        client: &RpcClient,
        pubkey: &Pubkey,
        payer_account: &Keypair,
        program_keypair: &Keypair,
    ) -> SolResult<()> {
        msg!("creating program instance at {}", pubkey);

        let contract_size = GreetingAccount::get_contract_size();

        let lamports = client.get_minimum_balance_for_rent_exemption(contract_size)?;

        msg!("minimim balance for rent exemption: {}", lamports);

        let instr = system_instruction::create_account_with_seed(
            &payer_account.pubkey(),
            &pubkey,
            &payer_account.pubkey(),
            SEED,
            lamports,
            contract_size as u64,
            &program_keypair.pubkey(),
        );

        let recent_blockhash = client.get_recent_blockhash()?.0;
        msg!("recent blockhash: {}", recent_blockhash);

        let tx = Transaction::new_signed_with_payer(
            &[instr],
            Some(&payer_account.pubkey()),
            &[payer_account],
            recent_blockhash,
        );

        let sig = client.send_and_confirm_transaction_with_spinner(&tx)?;

        msg!("account created");
        msg!("signature: {}", sig);

        Ok(())
    }
}
