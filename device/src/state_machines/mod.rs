use super::*;

pub mod no_session;
use no_session::NoSession;

pub mod session;
use session::Session;

pub struct Shared<R: radio::PhyRxTx + Timings> {
    radio: R,
    credentials: Credentials,
    region: RegionalConfiguration,
    mac: Mac,
    // TODO: do something nicer for randomness
    get_random: fn() -> u32,
    buffer: Vec<u8, U256>,
}

impl<R: radio::PhyRxTx + Timings> Shared<R> {
    pub fn new(
        radio: R,
        credentials: Credentials,
        region: RegionalConfiguration,
        mac: Mac,
        get_random: fn() -> u32,
        buffer: Vec<u8, U256>,
    ) -> Shared<R> {
        Shared {
            radio,
            credentials,
            region,
            mac,
            get_random,
            buffer,
        }
    }
}

pub enum State<R>
where
    R: radio::PhyRxTx + Timings,
{
    NoSession(NoSession<R>),
    Session(Session<R>),
}
