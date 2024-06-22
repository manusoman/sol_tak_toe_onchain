use solana_program::{
    program_error::ProgramError,
    sysvar::Sysvar,
    rent::Rent,
    clock::Clock
};

pub fn get_minimum_balance(account_size: u64) -> Result<u64, ProgramError> {
    let rent = Rent::get()?;
    Ok(rent.minimum_balance(account_size as usize))
}

pub fn get_starter() -> Result<u8, ProgramError> {
    let slot = Clock::get()?.slot;
    Ok((slot % 2) as u8)
}

pub fn should_invert(key1: &[u8], key2: &[u8]) -> Result<bool, ProgramError> {
    for i in 0..32 {
        if key1[i] < key2[i] { return Ok(false); }
        if key1[i] > key2[i] { return Ok(true); }
    }

    Err(ProgramError::InvalidAccountData)
}

pub fn same_keys(key1: &[u8], key2: &[u8]) -> bool {
    for i in 0..32 {
        if key1[i] != key2[i] { return false; }
    }

    true
}

pub fn copy_keys(source: &[u8], destination: &mut [u8]) {
    for i in 0..32 {
        destination[i] = source[i];
    }
}

pub fn did_win(moves: &[u8], limit: u8) -> bool {
    let mut diag_sum1: u8 = 0;
    let mut diag_sum2: u8 = 0;

    for i in 0..3 as usize {
        let mut row_sum: u8 = 0;
        let mut col_sum: u8 = 0;

        for j in 0..3 as usize {
            row_sum += moves[i * 3 + j];
            col_sum += moves[i + j * 3];
        }

        if row_sum == limit || col_sum == limit {
            return true;
        }

        diag_sum1 += moves[i * 3 + i];
        diag_sum2 += moves[i * 3 + (2 - i)];
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
