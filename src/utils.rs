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

pub fn did_win(moves: &[u8], start: u8, no_of_moves: u8) -> bool {
    let limit: u8 = 3;
    let mut diag_sum1: u8 = 0;
    let mut diag_sum2: u8 = 0;
    let mut plays: [u8; 9] = [0; 9];

    for i in (start..no_of_moves).step_by(2) {
        plays[moves[i as usize] as usize] = 1;
    }

    for i in 0..3 as usize {
        let mut row_sum: u8 = 0;
        let mut col_sum: u8 = 0;

        for j in 0..3 as usize {
            row_sum += plays[i * 3 + j];
            col_sum += plays[i + j * 3];
        }

        if row_sum == limit || col_sum == limit {
            return true;
        }

        diag_sum1 += plays[i * 3 + i];
        diag_sum2 += plays[i * 3 + (2 - i)];
    }

    if diag_sum1 == limit || diag_sum2 == limit {
        return true;
    }

    false
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
