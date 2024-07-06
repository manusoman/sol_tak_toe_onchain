mod utils;

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


const PLAYER_ACC_RANDOM_SEED: &[u8; 6] = b"player";
const GAME_ACC_RANDOM_SEED: &[u8; 4] = b"game";
const GAME_ACC_SIZE: u64 = 74;
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

    if !same_keys(&pda.to_bytes(), &player_pda_acc.key.to_bytes()) {
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
                &[wallet_acc.clone(), player_pda_acc.clone()],
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
            player_pda_acc.data.borrow_mut()[20] = if instruction_data[0] == 1 { 1 } else { 0 };
            Ok(())
        },

        3 => { // Invite someone for a game
            let opponent_pda_acc = next_account_info(acc_iter)?;
            let mut opponent_data = opponent_pda_acc.data.borrow_mut();

            if opponent_data[21] != 0 {
                return Err(ProgramError::InvalidAccountOwner);
            }

            let mut player_data = player_pda_acc.data.borrow_mut();

            opponent_data[21] = 1;
            player_data[21] = 2;
            copy_keys(&player_pda_acc.key.to_bytes(), &mut opponent_data[22..]);
            copy_keys(&opponent_pda_acc.key.to_bytes(), &mut player_data[22..]);

            Ok(())
        },

        4 => { // Accept/reject game invite
            let opponent_pda_acc = next_account_info(acc_iter)?;
            let mut player_data = player_pda_acc.data.borrow_mut();
            let mut opp_data = opponent_pda_acc.data.borrow_mut();
            
            if !same_keys(opponent_pda_acc.key.as_ref(), &player_data[22..]) {
                return Err(ProgramError::InvalidAccountData);
            }

            match instruction_data[1] {
                0 => { // Reject game invite
                    player_data[21..].fill(0);
                    opp_data[21..].fill(0);
                    Ok(())
                },

                1 => { // Accept game invite
                    let seed1 = player_pda_acc.key.as_ref();
                    let seed2 = opponent_pda_acc.key.as_ref();

                    let seeds = if should_invert(seed1, seed2)? {
                        [seed2, seed1, GAME_ACC_RANDOM_SEED]
                    } else {
                        [seed1, seed2, GAME_ACC_RANDOM_SEED]
                    };

                    let (game_pda, bump) = Pubkey::find_program_address(&seeds, program_id);
                    let game_pda_acc = next_account_info(acc_iter)?;

                    if !same_keys(game_pda.as_ref(), game_pda_acc.key.as_ref()) {
                        return Err(ProgramError::InvalidAccountData);
                    }

                    // Check if the game account already exists because of the improper handling
                    // from a perivous game. If yes, just reset it and use.
                    if game_pda_acc.lamports() > 0 {
                        game_pda_acc.data.borrow_mut().fill(0);
                    } else {
                        let ix = create_account(
                            wallet_acc.key,
                            &game_pda,
                            get_minimum_balance(GAME_ACC_SIZE)?,
                            GAME_ACC_SIZE,
                            program_id
                        );
    
                        invoke_signed(
                            &ix,
                            &[wallet_acc.clone(), game_pda_acc.clone()],
                            &[&[seeds[0], seeds[1], seeds[2], &[bump]]]
                        )?;
                    }

                    let mut game_data = game_pda_acc.data.borrow_mut();
                    let (first, second) = if get_starter() == 0
                    { (seed1, seed2) } else { (seed2, seed1) };

                    copy_keys(first, &mut game_data[0..32]);
                    copy_keys(second, &mut game_data[32..64]);
                    
                    player_data[21] = 3;
                    opp_data[21] = 3;
                    Ok(())
                },

                _ => Err(ProgramError::InvalidInstructionData)
            }
        },

        5 => { // User gameplay
            if instruction_data[1] > 8 { return Err(ProgramError::InvalidInstructionData); }

            let game_pda_acc = next_account_info(acc_iter)?;
            let mut game_data = game_pda_acc.data.borrow_mut();
            let no_of_moves = game_data[64] as usize;

            if no_of_moves >= 9 || game_data[65 + no_of_moves] != 0 {
                return Err(ProgramError::InvalidInstructionData);
            }

            let (key, start) = if game_data[64] % 2 == 0
            { (&game_data[..32], 0) } else { (&game_data[32..64], 1) };

            if !same_keys(player_pda_acc.key.as_ref(), key) {
                return Err(ProgramError::InvalidAccountData);
            }

            game_data[65 + no_of_moves] = instruction_data[1];
            game_data[64] += 1;

            // No need to check if it's a winning move unless
            // a minimum of 5 moves are made.
            if game_data[64] >= 5 {
                if did_win(&game_data[65..], start, game_data[64] as usize) {
                    game_data[64] = if start == 0 { 11 } else { 12 };
                } else if game_data[64] == 9 {
                    game_data[64] = 10;
                }
            }

            Ok(())
        },

        6 => { // Close game account
            Ok(())
        },

        7 => { // Close user account
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
