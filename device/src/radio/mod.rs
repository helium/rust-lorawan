use heapless::consts::*;
use heapless::Vec;

mod types;
pub use types::*;

use super::TimestampMs;

enum PhyResponse {
    Busy,
    TxDone(TimestampMs),
    RxDone(RxQuality),
    TxError,
    RxError,
}

pub enum PhyError {
    TxError,
    RxError,
}

mod state_machine;

pub use state_machine::{Error, Event, StateWrapper as State, Response};

pub trait PhyRxTx {
    type PhyEvent;
    fn send(&mut self, buffer: &mut [u8]);

    // we require mutability so we may decrypt in place
    fn get_received_packet(&mut self) -> &mut Vec<u8, U256>;

    fn cancel_tx(&mut self) -> Result<(), PhyError>;
    fn cancel_rx(&mut self) -> Result<(), PhyError>;

    fn configure_tx(&mut self, config: TxConfig);
    fn configure_rx(&mut self, config: RfConfig);
    fn set_rx(&mut self);

    fn handle_phy_event(&mut self, event: Self::PhyEvent) -> Option<PhyResponse>;
}
