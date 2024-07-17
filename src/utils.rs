use solana_program::{
    pubkey::Pubkey,
    program_error::ProgramError,
    sysvar::Sysvar,
    rent::Rent,
    clock::Clock
};

use super::{
    PLAYER_ACC_RANDOM_SEED,
    CHALLENGE_ACC_RANDOM_SEED,
    GAME_ACC_RANDOM_SEED
};


pub fn get_minimum_balance(account_size: u64) -> Result<u64, ProgramError> {
    let rent = Rent::get()?;
    Ok(rent.minimum_balance(account_size as usize))
}

pub fn get_timestamp() -> Result<u64, ProgramError> {
    let clock = Clock::get()?;
    Ok(clock.unix_timestamp as u64)
}

pub fn get_starter() -> u8 {
    if let Ok(clock) = Clock::get()
    { (clock.slot % 2) as u8 } else { 0 }
}

pub fn same_keys(key1: &[u8], key2: &[u8]) -> bool {
    for i in 0..32 {
        if key1[i] != key2[i] { return false; }
    }

    true
}

pub fn did_win(moves: &[u8], no_of_moves: u8) -> u8 {
    let mut plays: [u8; 9] = [0; 9];
    let start = if no_of_moves % 2 == 0 { 1 } else { 0 };
    let last_move = moves[(no_of_moves - 1) as usize];
    let row = (last_move / 3) as usize;
    let col = (last_move % 3) as usize;
    let limit = 3;
    let mut row_sum = 0;
    let mut col_sum = 0;
    let mut diag1_sum = 0;
    let mut diag2_sum = 0;

    for i in (start..no_of_moves).step_by(2) {
        plays[moves[i as usize] as usize] = 1;
    }

    for i in 0..3 as usize {
        row_sum += plays[row * 3 + i];
        col_sum += plays[i * 3 + col];
        diag1_sum += plays[4 * i];
        diag2_sum += plays[(2 - i) * 3 + i];
    }

    if row_sum == limit { return (row + 1) as u8; }
    if col_sum == limit { return (4 + col) as u8; }
    if diag1_sum == limit { return 7; }
    if diag2_sum == limit { return 8; }

    0
}

pub fn get_validated_name(buf: &[u8], min_len: usize) -> Result<String, ProgramError> {
    if buf.len() <= 20 {
        let temp = String::from_utf8(buf.to_vec());

        if temp.is_ok() {
            let name = temp.unwrap();
            let name = name.trim();

            if name.len() >= min_len {
                return Ok(String::from(name));
            }
        }
    }
    
    Err(ProgramError::InvalidInstructionData)
}

pub fn verify_player_acc(
    wallet_id: &[u8],
    player_acc_id: &[u8],
    bump: u8,
    program_id: &Pubkey
) -> bool {
    if let Ok(key) = Pubkey::create_program_address(
        &[wallet_id, PLAYER_ACC_RANDOM_SEED, &[bump]],
        program_id
    ) { return same_keys(key.as_ref(), player_acc_id); }

    false
}

pub fn verify_challenge_acc(
    player_acc_id: &[u8],
    opponent_acc_id: &[u8],
    challenge_acc_id: &[u8],
    bump: u8,
    program_id: &Pubkey
) -> bool {
    if let Ok(key) = Pubkey::create_program_address(
        &[player_acc_id, opponent_acc_id, CHALLENGE_ACC_RANDOM_SEED, &[bump]],
        program_id
    ) { return same_keys(key.as_ref(), challenge_acc_id); }

    false
}

pub fn verify_game_acc(
    challenge_acc_id: &[u8],
    game_acc_id: &[u8],
    bump: u8,
    program_id: &Pubkey
) -> bool {
    if let Ok(key) = Pubkey::create_program_address(
        &[challenge_acc_id, GAME_ACC_RANDOM_SEED, &[bump]],
        program_id
    ) { return same_keys(key.as_ref(), game_acc_id); }

    false
}

pub fn verify_game_players(
    player_id: &[u8],
    opponent_id: &[u8],
    game_acc_data: &[u8]
) -> bool {
    (player_id == &game_acc_data[..32] && opponent_id == &game_acc_data[32..64]) ||
    (opponent_id == &game_acc_data[..32] && player_id == &game_acc_data[32..64])
}
