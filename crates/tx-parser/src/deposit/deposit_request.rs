//! parser types for Deposit Tx, and later deposit Request Tx

use std::convert::TryInto;

use alpen_express_state::tx::DepositRequestInfo;
use bitcoin::{opcodes::all::OP_RETURN, ScriptBuf, Transaction};

use super::{
    common::{check_bridge_offer_output, DepositRequestScriptInfo},
    error::DepositParseError,
    DepositTxConfig,
};
use crate::utils::{next_bytes, next_op};

/// Extracts the DepositInfo from the Deposit Transaction
pub fn extract_deposit_request_info(
    tx: &Transaction,
    config: &DepositTxConfig,
) -> Option<DepositRequestInfo> {
    // Ensure that the transaction has at least 2 outputs
    let output_0 = tx.output.first()?;
    let output_1 = tx.output.get(1)?;

    // Parse the deposit request script from the second output's script_pubkey
    let DepositRequestScriptInfo {
        tap_ctrl_blk_hash,
        ee_bytes,
    } = parse_deposit_request_script(&output_1.script_pubkey, config).ok()?;

    // Check if the bridge offer output is valid
    check_bridge_offer_output(tx, config).ok()?;

    // Construct and return the DepositRequestInfo
    Some(DepositRequestInfo {
        amt: output_0.value.to_sat(),
        address: ee_bytes,
        take_back_leaf_hash: tap_ctrl_blk_hash,
    })
}

/// extracts the tapscript block and EE address given that the script is OP_RETURN type and
/// contains the Magic Bytes
pub fn parse_deposit_request_script(
    script: &ScriptBuf,
    config: &DepositTxConfig,
) -> Result<DepositRequestScriptInfo, DepositParseError> {
    let mut instructions = script.instructions();

    // check if OP_RETURN is present and if not just discard it
    if next_op(&mut instructions) != Some(OP_RETURN) {
        return Err(DepositParseError::NoOpReturn);
    }

    let Some(data) = next_bytes(&mut instructions) else {
        return Err(DepositParseError::NoData);
    };

    assert!(data.len() < 80);

    // data has expected magic bytes
    let magic_bytes = &config.magic_bytes;
    let magic_len = magic_bytes.len();
    if data.len() < magic_len || &data[..magic_len] != magic_bytes {
        return Err(DepositParseError::MagicBytesMismatch);
    }

    // 32 bytes of control hash
    let data = &data[magic_len..];
    if data.len() < 32 {
        return Err(DepositParseError::LeafHashLenMismatch);
    }
    let ctrl_hash: &[u8; 32] = &data[..32]
        .try_into()
        .expect("data length must be greater than 32");

    // configured bytes for address
    let address = &data[32..];
    if address.len() != config.address_length as usize {
        // casting is safe as address.len() < data.len() < 80
        return Err(DepositParseError::InvalidDestAddress(address.len() as u8));
    }

    Ok(DepositRequestScriptInfo {
        tap_ctrl_blk_hash: *ctrl_hash,
        ee_bytes: address.into(),
    })
}

#[cfg(test)]
mod tests {
    use bitcoin::{absolute::LockTime, Amount, Transaction};

    use super::extract_deposit_request_info;
    use crate::deposit::{
        deposit_request::parse_deposit_request_script,
        error::DepositParseError,
        test_utils::{
            build_no_op_deposit_request_script, build_test_deposit_request_script,
            create_transaction_two_outpoints, generic_taproot_addr, get_deposit_tx_config,
        },
    };

    #[test]
    fn check_deposit_parser() {
        // values for testing
        let config = get_deposit_tx_config();
        let amt = Amount::from_sat(config.deposit_quantity);
        let evm_addr = [1; 20];
        let dummy_control_block = [0xFF; 32];
        let generic_taproot_addr = generic_taproot_addr();

        let deposit_request_script = build_test_deposit_request_script(
            config.magic_bytes,
            dummy_control_block.to_vec(),
            evm_addr.to_vec(),
        );

        let test_transaction = create_transaction_two_outpoints(
            Amount::from_sat(config.deposit_quantity),
            &generic_taproot_addr.script_pubkey(),
            &deposit_request_script,
        );

        let out = extract_deposit_request_info(&test_transaction, &get_deposit_tx_config());

        assert!(out.is_some());
        let out = out.unwrap();

        assert_eq!(out.amt, amt.to_sat());
        assert_eq!(out.address, evm_addr);
        assert_eq!(out.take_back_leaf_hash, dummy_control_block);
    }

    #[test]
    fn test_invalid_script_no_op_return() {
        let evm_addr = [1; 20];
        let control_block = [0xFF; 65];

        let config = get_deposit_tx_config();
        let invalid_script = build_no_op_deposit_request_script(
            config.magic_bytes.clone(),
            control_block.to_vec(),
            evm_addr.to_vec(),
        );

        let out = parse_deposit_request_script(&invalid_script, &config);

        // Should return an error as there's no OP_RETURN
        assert!(matches!(out, Err(DepositParseError::NoOpReturn)));
    }

    #[test]
    fn test_invalid_evm_address_length() {
        let evm_addr = [1; 13]; // Invalid length EVM address
        let control_block = [0xFF; 32];

        let config = get_deposit_tx_config();

        let script = build_test_deposit_request_script(
            config.magic_bytes.clone(),
            control_block.to_vec(),
            evm_addr.to_vec(),
        );
        let out = parse_deposit_request_script(&script, &config);

        // Should return an error as EVM address length is invalid
        assert!(matches!(out, Err(DepositParseError::InvalidDestAddress(_))));
    }

    #[test]
    fn test_invalid_control_block() {
        let evm_addr = [1; 20];
        let control_block = [0xFF; 0]; // Missing control block

        let config = get_deposit_tx_config();
        let script_missing_control = build_test_deposit_request_script(
            config.magic_bytes.clone(),
            control_block.to_vec(),
            evm_addr.to_vec(),
        );

        let out = parse_deposit_request_script(&script_missing_control, &config);

        // Should return an error due to missing control block
        assert!(matches!(out, Err(DepositParseError::LeafHashLenMismatch)));
    }

    #[test]
    fn test_script_with_invalid_magic_bytes() {
        let evm_addr = [1; 20];
        let control_block = vec![0xFF; 32];
        let invalid_magic_bytes = vec![0x00; 4]; // Invalid magic bytes

        let config = get_deposit_tx_config();
        let invalid_script = build_test_deposit_request_script(
            invalid_magic_bytes,
            control_block,
            evm_addr.to_vec(),
        );

        let out = parse_deposit_request_script(&invalid_script, &config);

        // Should return an error due to invalid magic bytes
        assert!(matches!(out, Err(DepositParseError::MagicBytesMismatch)));
    }

    #[test]
    fn test_empty_transaction() {
        let config = get_deposit_tx_config();

        // Empty transaction with no outputs
        let test_transaction = Transaction {
            version: bitcoin::transaction::Version(2),
            lock_time: LockTime::ZERO,
            input: vec![],
            output: vec![],
        };

        let out = extract_deposit_request_info(&test_transaction, &config);

        // Should return an error as the transaction has no outputs
        assert!(out.is_none());
    }
}