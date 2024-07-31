use borsh::{BorshSerialize, BorshDeserialize};
use solana_program::account_info::AccountInfo;


#[derive(BorshSerialize, BorshDeserialize)]
pub struct GameData {
    pub player1: [u8; 32],
    pub player2: [u8; 32],
    pub no_of_moves: u8,
    pub game_status: u8,
    pub moves: [u8; 9] // Moves as box indeces [0 - 8]
}

impl GameData {
    pub fn parse(acc_info: &AccountInfo) -> Self {
        GameData::try_from_slice(
            *acc_info.data.borrow()
        ).unwrap()
    }

    pub fn clear(acc_info: &AccountInfo) {
        acc_info.data.borrow_mut().fill(0);
    }

    pub fn set_players(&mut self, id1: &[u8], id2: &[u8]) {
        self.player1.copy_from_slice(id1);
        self.player2.copy_from_slice(id2);
    }

    pub fn write(&self, acc_info: &AccountInfo) {
        self.serialize(&mut *acc_info.data.borrow_mut()).unwrap();
    }
}
