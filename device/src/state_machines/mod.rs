use super::*;
use lorawan_encoding::parser::{
    DecryptedDataPayload, DecryptedJoinAcceptPayload
};

pub mod no_session;
pub mod session;

pub struct Shared<R: radio::PhyRxTx + Timings, T> {
    radio: R,
    credentials: Credentials,
    region: region::Configuration,
    mac: Mac,
    // TODO: do something nicer for randomness
    get_random: fn() -> u32,
    buffer: Vec<u8, U256>,
    downlink: Option<Downlink<T>>,
}

enum Downlink<T> {
    Data(DecryptedDataPayload<Vec<u8, U256>>),
    Join(DecryptedJoinAcceptPayload<Vec<u8, U256>, T>),
}

impl<R: radio::PhyRxTx + Timings, C: CryptoFactory + Default> Shared<R, C> {
    pub fn get_mut_radio(&mut self) -> &mut R {
        &mut self.radio
    }
    pub fn get_mut_credentials(&mut self) -> &mut Credentials {
        &mut self.credentials
    }

    pub fn take_data_downlink(&mut self) -> Option<DecryptedDataPayload<Vec<u8, U256>>> {
        if let Some(Downlink::Data(data)) = self.downlink.take() {
            Some(data)
        } else{
            None
        }
    }

    pub fn take_join_accept(&mut self) -> Option<DecryptedJoinAcceptPayload<Vec<u8, U256>, C>> {
        if let Some(Downlink::Join(data)) = self.downlink.take() {
            Some(data)
        } else{
            None
        }
    }
}

impl<R: radio::PhyRxTx + Timings, C: CryptoFactory + Default> Shared<R, C> {
    pub fn new(
        radio: R,
        credentials: Credentials,
        region: region::Configuration,
        mac: Mac,
        get_random: fn() -> u32,
        buffer: Vec<u8, U256>,
    ) -> Shared<R, C> {
        Shared {
            radio,
            credentials,
            region,
            mac,
            get_random,
            buffer,
            downlink: None,
        }
    }
}

trait CommonState<R: radio::PhyRxTx + Timings, C: CryptoFactory + Default> {
    fn get_mut_shared(&mut self) -> &mut Shared<R, C>;
}
