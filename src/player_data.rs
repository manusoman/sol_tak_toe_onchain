use borsh::{BorshSerialize, BorshDeserialize};
use solana_program::account_info::AccountInfo;

#[derive(BorshSerialize, BorshDeserialize)]
pub struct PlayerData {
    name: [u8; 20],
    invitation_count: u8,
    pub current_game: [u8; 32]
}

impl PlayerData {
    pub fn parse(acc_info: &AccountInfo) -> Self {
        PlayerData::try_from_slice(
            *acc_info.data.borrow()
        ).unwrap()
    }

    pub fn clear(acc_info: &AccountInfo) {
        acc_info.data.borrow_mut().fill(0);
    }

    pub fn set_name(&mut self, data: &[u8]) {
        self.name.fill(32); // Fill name with space first
        self.name[..data.len()].copy_from_slice(data);
    }

    pub fn inc_invitation(&mut self) {
        self.invitation_count += 1;
    }

    pub fn dec_invitation(&mut self) {
        self.invitation_count -= 1;
    }

    pub fn set_current_game(&mut self, game: &[u8]) {
        self.current_game.copy_from_slice(game);
    }

    pub fn clear_current_game(&mut self) {
        self.current_game.fill(0);
    }

    pub fn write(&self, acc_info: &AccountInfo) {
        self.serialize(&mut *acc_info.data.borrow_mut());
    }
}
