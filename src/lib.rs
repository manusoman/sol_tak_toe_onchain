mod utils;
mod player_data;
mod challenge_data;
mod game_data;

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
    did_win,
    get_minimum_balance,
    get_starter,
    get_timestamp,
    get_validated_name,
    same_keys,
    verify_challenge_acc,
    verify_game_acc,
    verify_game_players,
    verify_player_acc,
    LamportManaged
};

use player_data::PlayerData;
use challenge_data::ChallengeData;
pub use game_data::GameData;

entrypoint!(process_instruction);


pub const PLAYER_ACC_RANDOM_SEED: &[u8; 6] = b"player";
const PLAYER_ACC_SIZE: u64 = 53;
const MIN_NAME_LENGTH: usize = 4;

pub const CHALLENGE_ACC_RANDOM_SEED: &[u8; 9] = b"challenge";
const CHALLENGE_ACC_SIZE: u64 = 73;

pub const GAME_ACC_RANDOM_SEED: &[u8; 4] = b"game";
const GAME_ACC_SIZE: u64 = 75;
const STAKES: [u64; 3] = [500_000_000, 1000_000_000, 2000_000_000];



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

            let mut player_data = PlayerData::parse(player_pda_acc);

            player_data.set_name(player_name.as_bytes());
            player_data.write(player_pda_acc);

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
            let stake_idx = instruction_data[3];

            // Only one challenge is possible for a given opponent at any moment.
            // Todo: make this clear to the player with a custom error.
            if challenge_pda_acc.lamports() > 0 || same_keys(
                player_pda_acc.key.as_ref(), opponent_pda_acc.key.as_ref()
            ) { return Err(ProgramError::InvalidAccountData); }

            if stake_idx > 2 { return Err(ProgramError::InvalidInstructionData); }

            if !verify_challenge_acc(
                player_pda_acc.key.as_ref(),
                opponent_pda_acc.key.as_ref(),
                challenge_pda_acc.key.as_ref(),
                challenge_pda_bump,
                program_id
            ) { return Err(ProgramError::InvalidAccountData); }

            let game_share = STAKES[stake_idx as usize];
            let balance = player_pda_acc.lamports() - get_minimum_balance(PLAYER_ACC_SIZE)?;
            if balance < game_share { return Err(ProgramError::InsufficientFunds); }

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

            challenge_pda_acc.set_lamports(game_share);
            player_pda_acc.set_lamports(player_pda_acc.lamports() - game_share);

            let mut challenge_data = ChallengeData::parse(challenge_pda_acc);
            let mut opponent_data = PlayerData::parse(opponent_pda_acc);

            challenge_data.set_players(
                opponent_pda_acc.key.as_ref(),
                player_pda_acc.key.as_ref()
            );

            challenge_data.stake_index = stake_idx;
            challenge_data.timestamp = get_timestamp()?;
            opponent_data.inc_invitation();

            challenge_data.write(challenge_pda_acc);
            opponent_data.write(opponent_pda_acc);
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

            let challenge_data = ChallengeData::parse(challenge_pda_acc);
            let game_share = STAKES[challenge_data.stake_index as usize];
            let balance = player_pda_acc.lamports() - get_minimum_balance(PLAYER_ACC_SIZE)?;
            
            if balance < game_share { return Err(ProgramError::InsufficientFunds); }

            let ix = create_account(
                wallet_acc.key,
                game_pda_acc.key,
                0,
                GAME_ACC_SIZE,
                program_id
            );

            invoke_signed(
                &ix,
                &[wallet_acc.clone(), game_pda_acc.clone()],
                &[&[
                    challenge_pda_acc.key.as_ref(),
                    GAME_ACC_RANDOM_SEED,
                    &[game_pda_bump]
                ]]
            )?;

            let mut game_data = GameData::parse(game_pda_acc);

            if get_starter() == 0 {
                game_data.set_players(&challenge_data.invited_id, &challenge_data.invitee_id);
            } else {
                game_data.set_players(&challenge_data.invitee_id, &challenge_data.invited_id);
            }

            challenge_pda_acc.set_lamports(0);
            player_pda_acc.set_lamports(player_pda_acc.lamports() - game_share);
            game_pda_acc.set_lamports(game_share * 2);

            let mut player_data = PlayerData::parse(player_pda_acc);
            let game_acc_id = game_pda_acc.key.as_ref();
            let opponent_pda_acc = next_account_info(acc_iter)?;
            let mut opponent_data = PlayerData::parse(opponent_pda_acc);

            player_data.dec_invitation();
            player_data.set_current_game(game_acc_id);
            opponent_data.set_current_game(game_acc_id);

            player_data.write(player_pda_acc);
            opponent_data.write(opponent_pda_acc);
            ChallengeData::clear(challenge_pda_acc);
            game_data.write(game_pda_acc);
            
            Ok(())
        },

        5 => { // User gameplay
            let player_data = PlayerData::parse(player_pda_acc);
            let game_pda_acc = next_account_info(acc_iter)?;
            let mut game_data = GameData::parse(game_pda_acc);
            let box_idx = instruction_data[2];
            let no_of_moves = game_data.no_of_moves as usize;

            let key = if no_of_moves % 2 == 0
            { &game_data.player1 } else { &game_data.player2 };

            if game_pda_acc.key.as_ref() != &player_data.current_game ||
               key != player_pda_acc.key.as_ref() {
                return Err(ProgramError::InvalidAccountData);
            }

            if box_idx > 8 || no_of_moves == 9 || game_data.game_status > 0 {
                return Err(ProgramError::InvalidInstructionData);
            }

            // Ensure the same box_idx value doesn't already exist.
            for i in 0..no_of_moves {
                if box_idx == game_data.moves[i] {
                    return Err(ProgramError::InvalidInstructionData);
                }
            }

            game_data.moves[no_of_moves] = box_idx;
            game_data.no_of_moves += 1;

            // No need to check if it's a winning move unless
            // a minimum of 5 moves are made.
            if game_data.no_of_moves >= 5 {
                let res = did_win(&game_data.moves, game_data.no_of_moves);
                game_data.game_status = if game_data.no_of_moves == 9 && res == 0 { 9 } else { res };
            }

            game_data.write(game_pda_acc);
            Ok(())
        },

        6 => { // Close game
            let game_pda_acc = next_account_info(acc_iter)?;
            let game_data = GameData::parse(game_pda_acc);
            let opponent_pda_acc = next_account_info(acc_iter)?;
            
            let game_lamports = game_pda_acc.lamports();
            if game_lamports == 0 { return Ok(()); }

            if !verify_game_players(
                player_pda_acc.key.as_ref(),
                opponent_pda_acc.key.as_ref(),
                &game_data
            ) { return Err(ProgramError::InvalidAccountData); }

            match game_data.game_status {
                0 => opponent_pda_acc.add_lamports(game_lamports),

                9 => {
                    let half = game_lamports / 2;
                    player_pda_acc.add_lamports(half);
                    opponent_pda_acc.add_lamports(half);
                },

                _ => {
                    let temp_acc = if game_data.no_of_moves % 2 == 0 {
                        if player_pda_acc.key.as_ref() == &game_data.player2
                        { player_pda_acc } else { opponent_pda_acc }
                    } else {
                        if player_pda_acc.key.as_ref() == &game_data.player1
                        { player_pda_acc } else { opponent_pda_acc }
                    };

                    temp_acc.add_lamports(game_lamports);
                }
            }

            let mut player_data = PlayerData::parse(player_pda_acc);
            let mut opponent_data = PlayerData::parse(opponent_pda_acc);

            game_pda_acc.set_lamports(0);
            GameData::clear(game_pda_acc);
            player_data.clear_current_game();
            opponent_data.clear_current_game();
            
            player_data.write(player_pda_acc);
            opponent_data.write(opponent_pda_acc);
            Ok(())
        },

        7 => { // Transfer player account balance to player wallet
            let min_balance = get_minimum_balance(PLAYER_ACC_SIZE)?;

            player_pda_acc.set_lamports(min_balance);
            wallet_acc.add_lamports(player_pda_acc.lamports() - min_balance);

            Ok(())
        },

        8 => { // Close user account
            // Transfer balance to wallet
            wallet_acc.add_lamports(player_pda_acc.lamports());
            player_pda_acc.set_lamports(0);

            // Reset the data
            PlayerData::clear(player_pda_acc);

            msg!("Game account [{}] closed", player_pda_acc.key.to_string());
            Ok(())
        },

        _ => Err(ProgramError::InvalidInstructionData)
    }
}
