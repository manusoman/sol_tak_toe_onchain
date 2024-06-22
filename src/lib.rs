mod utils;

use core::str::FromStr;
use solana_program::{
    account_info::{AccountInfo, next_account_info},
    entrypoint,
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
    program::invoke_signed,
    system_instruction::create_account,
    msg
};

use utils::{
    get_minimum_balance,
    get_starter,
    should_invert,
    same_keys,
    copy_keys,
    did_win,
    get_validated_name
};

entrypoint!(process_instruction);


const WALLET_SEED: &[u8; 6] = b"wallet";
const WALLET_BUMP: u8 = 253;
const WALLET_ID: &str = "4mcytn4oSgYxz93CPXnMYcDWByyhC1oHQkUWQb9jVUJr";
const PLAYER_ACC_RANDOM_SEED: &[u8; 6] = b"player";
const GAME_ACC_RANDOM_SEED: &[u8; 4] = b"game";
const PLAYER_ACC_SIZE: u64 = 54;
const MIN_NAME_LENGTH: usize = 4;


pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8]
) -> ProgramResult {
    let acc_iter = &mut accounts.iter();
    let wallet_acc = next_account_info(acc_iter)?;
    let player_pda_acc = next_account_info(acc_iter)?;

    let (pda, bump) = Pubkey::find_program_address(
        &[&wallet_acc.key.to_bytes(), PLAYER_ACC_RANDOM_SEED],
        program_id
    );

    if pda.to_bytes() != player_pda_acc.key.to_bytes() {
        return Err(ProgramError::InvalidAccountData);
    }
    
    match instruction_data[0] {
        0 => { // User registration
            let name_buf = &instruction_data[1..];
            let player_name = get_validated_name(name_buf, MIN_NAME_LENGTH)?;

            let ix = create_account(
                wallet_acc.key,
                &pda,
                get_minimum_balance(PLAYER_ACC_SIZE)?,
                PLAYER_ACC_SIZE,
                program_id
            );

            invoke_signed(
                &ix, 
                &[
                    wallet_acc.clone(),
                    player_pda_acc.clone()
                ],
                &[&[&wallet_acc.key.to_bytes(), PLAYER_ACC_RANDOM_SEED, &[bump]]]
            )?;

            let mut acc_data = player_pda_acc.data.borrow_mut();

            acc_data[0..20].fill(32); // First fill the space with character ' ' (space).
            acc_data[0..player_name.len()].copy_from_slice(player_name.as_bytes());
            acc_data[20] = 1;

            msg!("Game account [{}] created for: {}", pda.to_string(), wallet_acc.key.to_string());
            Ok(())
        },

        1 | 2 => { // User login/logout
            player_pda_acc.data.borrow_mut()[0] = if instruction_data[0] == 1 { 1 } else { 0 };
            Ok(())
        },

        3 => { // Invite someone for a game
            let opponent_pda_acc = next_account_info(acc_iter)?;
            let mut opponent_data = opponent_pda_acc.data.borrow_mut();

            if opponent_data[1] != 0 {
                return Err(ProgramError::InvalidAccountOwner);
            }

            let mut player_data = player_pda_acc.data.borrow_mut();

            opponent_data[1] = 1;
            player_data[1] = 1;
            copy_keys(&player_pda_acc.key.to_bytes(), &mut opponent_data[2..]);
            copy_keys(&opponent_pda_acc.key.to_bytes(), &mut player_data[2..]);

            Ok(())
        },

        4 => { // Accept/reject game invite
            let opponent_pda_acc = next_account_info(acc_iter)?;
            let mut opp_data = player_pda_acc.data.borrow_mut();
            
            if !same_keys(&opponent_pda_acc.key.to_bytes(), &opp_data[2..]) {
                return Err(ProgramError::InvalidAccountData);
            }

            match instruction_data[1] {
                0 => { // Reject game invite
                    opp_data[1..].fill(0);
                    player_pda_acc.data.borrow_mut()[1..].fill(0);
                    Ok(())
                },

                1 => { // Accept game invite
                    let seed1 = &player_pda_acc.key.to_bytes();
                    let seed2 = &opponent_pda_acc.key.to_bytes();
                    let program_wallet = &Pubkey::from_str(WALLET_ID).unwrap();
                    let game_account_size: u64 = 74;

                    let (game_pda, _) = if should_invert(seed1, seed2)? {
                        Pubkey::find_program_address(&[seed2, seed1, GAME_ACC_RANDOM_SEED], program_id)
                    } else {
                        Pubkey::find_program_address(&[seed1, seed2, GAME_ACC_RANDOM_SEED], program_id)
                    };

                    let ix = create_account(
                        program_wallet,
                        &game_pda,
                        get_minimum_balance(game_account_size)?,
                        game_account_size,
                        program_id
                    );

                    let program_wallet_acc = next_account_info(acc_iter)?;
                    let game_pda_acc = next_account_info(acc_iter)?;

                    invoke_signed(
                        &ix,
                        &[
                            program_wallet_acc.clone(),
                            game_pda_acc.clone()
                        ],
                        &[&[WALLET_SEED, &[WALLET_BUMP]]]
                    )?;

                    let mut game_data = game_pda_acc.data.borrow_mut();

                    let (first, second) = if get_starter()? == 0 {
                        (seed1, seed2)
                    } else {
                        (seed2, seed1)
                    };

                    copy_keys(first, &mut game_data[0..32]);
                    copy_keys(second, &mut game_data[32..64]);
                    
                    opp_data[1] = 2;
                    player_pda_acc.data.borrow_mut()[1] = 2;
                    Ok(())
                },

                _ => Err(ProgramError::InvalidInstructionData)
            }
        },

        5 => { // User gameplay
            let idx = instruction_data[1] as usize;
            let game_pda_acc = next_account_info(acc_iter)?;
            let mut game_data = game_pda_acc.data.borrow_mut();

            if idx > 8 || game_data[64] > 8 || game_data[65 + idx] != 0 {
                return Err(ProgramError::InvalidInstructionData);
            }

            let (key, n) = if game_data[64] % 2 == 0 { (&game_data[..32], 1 as u8) }
            else { (&game_data[32..64], 10 as u8) };

            if !same_keys(&wallet_acc.key.to_bytes(), key) {
                return Err(ProgramError::InvalidAccountData);
            }

            game_data[65 + idx] = n;
            game_data[64] += 1;

            if game_data[64] > 4 {
                if did_win(&game_data[65..], n * 3) {
                    game_data[64] = if n == 1 { 11 } else { 12 };
                } else if game_data[64] == 9 {
                    game_data[64] = 10;
                }
            }

            Ok(())
        },

        6 => { // User account close
            let wallet_balance = wallet_acc.lamports();

            // Transfer balance to wallet
            **wallet_acc.lamports.borrow_mut() = wallet_balance.checked_add(player_pda_acc.lamports()).unwrap();
            **player_pda_acc.lamports.borrow_mut() = 0;

            // Reset the data
            player_pda_acc.data.borrow_mut().fill(0);

            msg!("Game account [{}] closed", player_pda_acc.key.to_string());
            Ok(())
        },

        _ => Err(ProgramError::InvalidInstructionData)
    }
}
