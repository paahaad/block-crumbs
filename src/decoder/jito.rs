use std::{collections::HashSet, sync::LazyLock};

use solana_pubkey::Pubkey;
use borsh::BorshDeserialize;

// RPC Method: "getTipAccounts", https://mainnet.block-engine.jito.wtf/api/v1/getTipAccounts
pub static JITO_TIP_ACCOUNTS: LazyLock<HashSet<Pubkey>> = LazyLock::new(||{
    [
        "96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5",
        "HFqU5x63VTqvQss8hp11i4wVV8bD44PvwucfZ2bU7gRe",
        "Cw8CFyM9FkoMi7K7Crf6HNQqf4uEMzpKw6QNghXLvLkY",
        "ADaUMid9yfUytqMBgopwjb2DTLSokTSzL1zt6iGPaS49",
        "DfXygSm4jCyNCybVYYK6DwvWqjKee8pbDmJGcLWNDXjh",
        "ADuUkR4vqLUMWXxW9gh6D6L8pMSawimctcNZ5pGwDcEt",
        "DttWaMuVvTiduZRnguLF7jNxTgiMBZ1hyAumKUiL2KRL",
        "3AVi9Tg9Uo68tJfuvoKvqKNWKkC5wPdSSdeBnizKZ6jT",
    ]
    .iter()
    .map(|s| Pubkey::from_str_const(s))
    .collect()
});

pub const SYSTEM_PROGRAM_ID: Pubkey = solana_pubkey::Pubkey::from_str_const("11111111111111111111111111111111");


#[derive(Debug, Clone, Default)]
pub struct JitoTipInfo {
    pub is_jito_bundle: bool,
    pub tip_amount: Option<u64>,
    pub tip_account: Option<Pubkey>
}

#[derive(BorshDeserialize)]
struct SystemTransfer {
    discriminator: u32,
    amount: u64,
}

impl JitoTipInfo {
    pub fn from_ixs(ixs: &[solana_instruction::Instruction]) -> Self {

        for ix in ixs {

            if ix.program_id != SYSTEM_PROGRAM_ID {
                continue;
            }

            if ix.accounts.len() < 2 {
                continue;
            }

            let destination = ix.accounts[1].pubkey;

            if JITO_TIP_ACCOUNTS.contains(&destination) {
                if let Ok(transfer) = SystemTransfer::try_from_slice(&ix.data) {
                    if transfer.discriminator == 2 {
                        return JitoTipInfo { is_jito_bundle: true, tip_amount: Some(transfer.amount), tip_account: Some(destination) };
                    }
                }
            }
        }


        JitoTipInfo::default()
    }
}


// landing method

pub enum LandingMethod {
    JitoBundle,
    PriorityFee,
    BaseFee
}

impl LandingMethod {
    pub fn as_str(&self) -> &str {
        match self {
            LandingMethod::JitoBundle => "jito_bundle",
            LandingMethod::PriorityFee => "priority_fee",
            LandingMethod::BaseFee => "base_fee"
        }
    }
}

pub fn classify_landing_method(
    is_jito_bundle: bool,
    compute_unit_price: Option<u64>,
) -> LandingMethod {
    if is_jito_bundle {
        LandingMethod::JitoBundle
    } else if compute_unit_price.map(|p| p > 0).unwrap_or(false) {
        LandingMethod::PriorityFee
    } else {
        LandingMethod::BaseFee
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use solana_instruction::{AccountMeta, Instruction};

    #[test]
    fn test_jito_tip_detection() {
        let jito_account =
            Pubkey::from_str_const("96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5");
        let sender = Pubkey::new_unique();

        // Create a transfer instruction to Jito tip account
        let mut data = vec![0x02, 0x00, 0x00, 0x00]; // Transfer discriminator
        data.extend_from_slice(&1_000_000u64.to_le_bytes()); // 1M lamports

        let ix = Instruction {
            program_id: SYSTEM_PROGRAM_ID,
            accounts: vec![
                AccountMeta::new(sender, true),
                AccountMeta::new(jito_account, false),
            ],
            data,
        };

        let info = JitoTipInfo::from_ixs(&[ix]);
        assert!(info.is_jito_bundle);
        assert_eq!(info.tip_amount, Some(1_000_000));
        assert_eq!(info.tip_account, Some(jito_account));
    }

    #[test]
    fn test_non_jito_transfer() {
        let random_account = Pubkey::new_unique();
        let sender = Pubkey::new_unique();

        let mut data = vec![0x02, 0x00, 0x00, 0x00];
        data.extend_from_slice(&1_000_000u64.to_le_bytes());

        let ix = Instruction {
            program_id: SYSTEM_PROGRAM_ID,
            accounts: vec![
                AccountMeta::new(sender, true),
                AccountMeta::new(random_account, false),
            ],
            data,
        };

        let info = JitoTipInfo::from_ixs(&[ix]);
        assert!(!info.is_jito_bundle);
        assert_eq!(info.tip_amount, None);
    }
}