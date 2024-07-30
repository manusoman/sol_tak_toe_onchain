use borsh::{BorshSerialize, BorshDeserialize};
use solana_program::account_info::AccountInfo;


#[derive(BorshSerialize, BorshDeserialize)]
pub struct ChallengeData {
    pub invited_id: [u8; 32],
    pub invitee_id: [u8; 32],
    pub stake_index: u8,
    pub timestamp: u64
}

impl ChallengeData {
    pub fn parse(acc_info: &AccountInfo) -> Self {
        ChallengeData::try_from_slice(
            *acc_info.data.borrow()
        ).unwrap()
    }

    pub fn clear(acc_info: &AccountInfo) {
        acc_info.data.borrow_mut().fill(0);
    }

    pub fn set_players(&mut self, invited: &[u8], invitee: &[u8]) {
        self.invited_id.copy_from_slice(invited);
        self.invitee_id.copy_from_slice(invitee);
    }

    pub fn write(&self, acc_info: &AccountInfo) {
        self.serialize(&mut *acc_info.data.borrow_mut());
    }
}
