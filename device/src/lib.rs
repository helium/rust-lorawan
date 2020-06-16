#![no_std]

use heapless::consts::*;
use heapless::Vec;

mod radio;

mod mac;
use mac::Mac;

mod types;
pub use types::*;

mod us915;
use us915::Configuration as RegionalConfiguration;

mod state_machines;
use state_machines::{no_session, session};

pub struct Device<R: radio::PhyRxTx + Timings> {
    state: State<R>,
    shared: Shared<R>,
}

pub struct Shared <R: radio::PhyRxTx + Timings> {
    radio: radio::State<R>,
    credentials: Credentials,
    region: RegionalConfiguration,
    mac: Mac,
    // TODO: do something nicer for randomness
    get_random: fn() -> u32,
    buffer: Vec<u8, U256>,
}

pub enum Response {
    Rx,         // packet received
    TxComplete, // packet sent
}

pub enum Error<'a, R>
where
    R: radio::PhyRxTx,
{
    RadioError(radio::Error<'a, R>), // error: unhandled event
    SessionError(session::Error),
    NoSessionError(no_session::Error),
}

pub enum Event<'a, R>
where
    R: radio::PhyRxTx,
{
    NewSession,
    RadioEvent(radio::Event<'a, R>),
    Timeout,
}

pub enum State<R>
where
    R: radio::PhyRxTx + Timings,
{
    NoSession(no_session::NoSession<R>),
    Session(session::Session<R>),
}

use core::default::Default;
impl<R> Default for State<R>
    where
        R: radio::PhyRxTx + Timings,
{
    fn default() -> Self {
        State::NoSession(no_session::NoSession::default())
    }
}

pub trait Timings {
    fn get_rx_window_offset_ms(&mut self) -> isize;
    fn get_rx_window_duration_ms(&mut self) -> usize;
}

impl<R: radio::PhyRxTx + Timings> Device<R> {
    pub fn new(
        deveui: [u8; 8],
        appeui: [u8; 8],
        appkey: [u8; 16],
        get_random: fn() -> u32,
    ) -> Device<R> {
        let mut region = RegionalConfiguration::new();
        region.set_subband(2);

        Device {
            state: State::default(),
            shared: Shared {
                radio: radio::State::default(),
                credentials: Credentials::new(deveui, appeui, appkey),
                region,
                get_random,
                mac: Mac::default(),
                buffer: Vec::new(),
            }
        }
    }

    pub fn send<E: radio::PhyRxTx>(
        &mut self,
        radio: &mut R,
        data: &[u8],
        fport: u8,
        confirmed: bool,
    ) {
        // let mut random = 0;//(self.get_random)();
        // let frequency = 0;//self.region.get_join_frequency(random as u8);
        //
        // let event: radio::Event<R> = radio::Event::TxRequest(
        //     radio::TxConfig {
        //         pw: 20,
        //         rf: radio::RfConfig {
        //             frequency,
        //             bandwidth: radio::Bandwidth::_125KHZ,
        //             spreading_factor: radio::SpreadingFactor::_10,
        //             coding_rate: radio::CodingRate::_4_5,
        //         },
        //     },
        //     &mut self.buffer,
        // );
        //
        // if let Ok(Some(response)) = self.radio.handle_event(radio, event) {
        //     // deal with response
        // };
    }

    pub fn handle_event<'a>(
        mut self,
        radio: &mut R,
        event: Event<'a, R>,
    ) -> (Self, Result<Option<Response>, Error<'a, R>>) {
        let (new_state, response) = match self.state {
            State::NoSession(state) => state.handle_event(&mut self.shared, radio, event),
            State::Session(state) => state.handle_event(&mut self.shared, radio, event),
        };
        self.state = new_state;
        (self, response)
    }
}
