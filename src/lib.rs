mod utils;

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction::create_account
};

use utils::{
    verify_player_acc,
    verify_challenge_acc,
    verify_game_acc,
    verify_game_players,
    get_minimum_balance,
    get_timestamp,
    get_starter,
    same_keys,
    did_win,
    get_validated_name
};

entrypoint!(process_instruction);


pub const PLAYER_ACC_RANDOM_SEED: &[u8; 6] = b"player";
const PLAYER_ACC_SIZE: u64 = 53;
const MIN_NAME_LENGTH: usize = 4;

pub const CHALLENGE_ACC_RANDOM_SEED: &[u8; 9] = b"challenge";
const CHALLENGE_ACC_SIZE: u64 = 72;

pub const GAME_ACC_RANDOM_SEED: &[u8; 4] = b"game";
const GAME_ACC_SIZE: u64 = 75;
const GAME_SHARE: u64 = 500_000_000; // 0.5 Sols



pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8]
) -> ProgramResult {
    let acc_iter = &mut accounts.iter();
    let wallet_acc = next_account_info(acc_iter)?;
    let player_pda_acc = next_account_info(acc_iter)?;
    let player_pda_acc_bump = instruction_data[0];

    if !verify_player_acc(
        wallet_acc.key.as_ref(),
        player_pda_acc.key.as_ref(),
        player_pda_acc_bump,
        program_id
    ) {
        return Err(ProgramError::InvalidAccountData);
    }
    
    match instruction_data[1] {
        0 => { // User registration
            let name_buf = &instruction_data[2..];
            let player_name = get_validated_name(name_buf, MIN_NAME_LENGTH)?;

            let ix = create_account(
                wallet_acc.key,
                player_pda_acc.key,
                get_minimum_balance(PLAYER_ACC_SIZE)?,
                PLAYER_ACC_SIZE,
                program_id
            );

            invoke_signed(
                &ix, 
                &[wallet_acc.clone(), player_pda_acc.clone()],
                &[&[wallet_acc.key.as_ref(), PLAYER_ACC_RANDOM_SEED, &[player_pda_acc_bump]]]
            )?;

            let mut acc_data = player_pda_acc.data.borrow_mut();

            acc_data[0..20].fill(32); // First fill the space with character ' ' (space).
            acc_data[0..player_name.len()].copy_from_slice(player_name.as_bytes());

            msg!("Game account created [{}]", player_pda_acc.key.to_string());
            Ok(())
        },

        1 | 2 => { // User login/logout
            // Is it needed?
            // If yes, how to implement it?
            Ok(())
        },

        3 => { // Challenge someone for a game
            let opponent_pda_acc = next_account_info(acc_iter)?;
            let challenge_pda_acc = next_account_info(acc_iter)?;
            let challenge_pda_bump = instruction_data[2];

            // Only one challenge is possible for a given opponent at any moment.
            // Todo: make this clear to the player with a custom error.
            if challenge_pda_acc.lamports() > 0 {
                return Err(ProgramError::InvalidAccountData);
            }

            if same_keys(player_pda_acc.key.as_ref(), opponent_pda_acc.key.as_ref()) {
                return Err(ProgramError::InvalidAccountData);
            }

            if !verify_challenge_acc(
                player_pda_acc.key.as_ref(),
                opponent_pda_acc.key.as_ref(),
                challenge_pda_acc.key.as_ref(),
                challenge_pda_bump,
                program_id
            ) { return Err(ProgramError::InvalidAccountData); }

            let balance = player_pda_acc.lamports() - get_minimum_balance(PLAYER_ACC_SIZE)?;
            if balance < GAME_SHARE { return Err(ProgramError::InsufficientFunds); }

            let ix = create_account(
                wallet_acc.key,
                challenge_pda_acc.key,
                0,
                CHALLENGE_ACC_SIZE,
                program_id
            );

            invoke_signed(
                &ix,
                &[wallet_acc.clone(), challenge_pda_acc.clone()],
                &[&[
                    player_pda_acc.key.as_ref(),
                    opponent_pda_acc.key.as_ref(),
                    CHALLENGE_ACC_RANDOM_SEED,
                    &[challenge_pda_bump]
                ]]
            )?;

            **challenge_pda_acc.lamports.borrow_mut() = GAME_SHARE;
            **player_pda_acc.lamports.borrow_mut() = player_pda_acc.lamports() - GAME_SHARE;

            let mut challenge_acc_data = challenge_pda_acc.data.borrow_mut();
            let timestamp = get_timestamp()?;

            challenge_acc_data[..32].copy_from_slice(opponent_pda_acc.key.as_ref());
            challenge_acc_data[32..64].copy_from_slice(player_pda_acc.key.as_ref());
            challenge_acc_data[64..].copy_from_slice(&timestamp.to_be_bytes());
            opponent_pda_acc.data.borrow_mut()[20] += 1;

            Ok(())
        },

        4 => { // Accept game invite
            let challenge_pda_acc = next_account_info(acc_iter)?;
            let game_pda_acc = next_account_info(acc_iter)?;
            let game_pda_bump = instruction_data[2];

            if !verify_game_acc(
                challenge_pda_acc.key.as_ref(),
                game_pda_acc.key.as_ref(),
                game_pda_bump,
                program_id
            ) { return Err(ProgramError::InvalidAccountData); }

            let balance = player_pda_acc.lamports() - get_minimum_balance(PLAYER_ACC_SIZE)?;
            if balance < GAME_SHARE { return Err(ProgramError::InsufficientFunds); }

            let ix = create_account(
                wallet_acc.key,
                game_pda_acc.key,
                0,
                GAME_ACC_SIZE,
                program_id
            );

            invoke_signed(
                &ix,
                &[wallet_acc.clone(), challenge_pda_acc.clone()],
                &[&[
                    challenge_pda_acc.key.as_ref(),
                    GAME_ACC_RANDOM_SEED,
                    &[game_pda_bump]
                ]]
            )?;

            let mut challenge_acc_data = challenge_pda_acc.data.borrow_mut();
            let mut game_acc_data = game_pda_acc.data.borrow_mut();

            if get_starter() == 0 {
                game_acc_data[..64].copy_from_slice(&challenge_acc_data[..64]);
            } else {
                game_acc_data[..32].copy_from_slice(&challenge_acc_data[32..64]);
                game_acc_data[32..64].copy_from_slice(&challenge_acc_data[..32]);
            }

            challenge_acc_data.fill(0);
            **challenge_pda_acc.lamports.borrow_mut() = 0;
            **player_pda_acc.lamports.borrow_mut() = player_pda_acc.lamports() - GAME_SHARE;
            **game_pda_acc.lamports.borrow_mut() = GAME_SHARE * 2;

            let opponent_pda_acc = next_account_info(acc_iter)?;
            let game_acc_id = game_pda_acc.key.as_ref();
            let mut player_acc_data = player_pda_acc.data.borrow_mut();

            player_acc_data[20] -= 1;
            player_acc_data[21..].copy_from_slice(game_acc_id);
            opponent_pda_acc.data.borrow_mut()[21..].copy_from_slice(game_acc_id);

            Ok(())
        },

        5 => { // User gameplay
            let game_pda_acc = next_account_info(acc_iter)?;
            let player_acc_id = player_pda_acc.key.as_ref();
            let mut game_data = game_pda_acc.data.borrow_mut();

            if game_pda_acc.key.as_ref() != &player_pda_acc.data.borrow()[21..] ||
                (player_acc_id != &game_data[..32] && player_acc_id != &game_data[32..64]) {
                return Err(ProgramError::InvalidAccountData);
            }

            let box_idx = instruction_data[2];
            let no_of_moves = game_data[64] as usize;

            let (key, start) = if no_of_moves % 2 == 0
            { (&game_data[..32], 0) } else { (&game_data[32..64], 1) };

            if key != player_pda_acc.key.as_ref() {
                return Err(ProgramError::InvalidAccountData);
            }

            if box_idx > 8 || no_of_moves >= 9 { return Err(ProgramError::InvalidInstructionData); }

            // Ensure the same box_idx value doesn't already exist.
            for i in 0..no_of_moves {
                if box_idx == game_data[66 + i] {
                    return Err(ProgramError::InvalidInstructionData);
                }
            }

            game_data[66 + no_of_moves] = box_idx;
            game_data[64] += 1;

            // No need to check if it's a winning move unless
            // a minimum of 5 moves are made.
            if game_data[64] >= 5 {
                if did_win(&game_data[66..], start, game_data[64]) {
                    game_data[65] = start + 1;
                } else if game_data[64] == 9 {
                    game_data[65] = 3;
                }
            }

            Ok(())
        },

        6 => { // Close game
            let opponent_pda_acc = next_account_info(acc_iter)?;
            let game_pda_acc = next_account_info(acc_iter)?;
            let mut game_data = game_pda_acc.data.borrow_mut();

            if !verify_game_players(
                player_pda_acc.key.as_ref(),
                opponent_pda_acc.key.as_ref(),
                &game_data[..64]
            ) { return Err(ProgramError::InvalidAccountData); }

            
            let game_acc_balance = game_pda_acc.lamports();
            if game_acc_balance == 0 { return Ok(()); }

            match game_data[65] {
                0 => {
                    **opponent_pda_acc.lamports.borrow_mut() = opponent_pda_acc.lamports().checked_add(game_acc_balance).unwrap();
                },

                1 => {
                    let temp_acc = if player_pda_acc.key.as_ref() == &game_data[..32]
                    { player_pda_acc } else { opponent_pda_acc };

                    **temp_acc.lamports.borrow_mut() = temp_acc.lamports().checked_add(game_acc_balance).unwrap();
                },

                2 => {
                    let temp_acc = if player_pda_acc.key.as_ref() == &game_data[32..64]
                    { player_pda_acc } else { opponent_pda_acc };

                    **temp_acc.lamports.borrow_mut() = temp_acc.lamports().checked_add(game_acc_balance).unwrap();
                },

                3 => {
                    let half = game_acc_balance / 2;
                    **player_pda_acc.lamports.borrow_mut() = player_pda_acc.lamports().checked_add(half).unwrap();
                    **opponent_pda_acc.lamports.borrow_mut() = opponent_pda_acc.lamports().checked_add(half).unwrap();
                },

                _ => return Err(ProgramError::Custom(0))
            }

            **game_pda_acc.lamports.borrow_mut() = 0;
            game_data.fill(0);
            player_pda_acc.data.borrow_mut()[21..].fill(0);
            opponent_pda_acc.data.borrow_mut()[21..].fill(0);
            
            Ok(())
        },

        7 => { // Transfer player account balance to player wallet
            let min_balance = get_minimum_balance(PLAYER_ACC_SIZE)?;
            let extra = player_pda_acc.lamports() - min_balance;

            **player_pda_acc.lamports.borrow_mut() = min_balance;
            **wallet_acc.lamports.borrow_mut() = wallet_acc.lamports().checked_add(extra).unwrap();

            Ok(())
        },

        8 => { // Close user account
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
