use super::super::*;
use core::marker::PhantomData;
use lorawan_encoding::{
    self,
    creator::{JoinRequestCreator},
    keys::AES128,
    parser::DevAddr,
    parser::{parse as lorawan_parse, *},
};

pub enum State<R>
where
    R: radio::PhyRxTx + Timings,
{
    Idle(Idle<R>),
    SendingJoin(SendingJoin<R>),
    WaitingForRxWindow(WaitingForRxWindow<R>),
    WaitingForJoinResponse(WaitingForJoinResponse<R>),
}

pub enum Error {}

type DevNonce = lorawan_encoding::parser::DevNonce<[u8; 2]>;

pub struct NoSession<R>
where
    R: radio::PhyRxTx + Timings
{
    state: State<R>,
}

impl<R> NoSession<R>
    where
        R: radio::PhyRxTx + Timings
{
    pub fn handle_event<'a>(
        mut self,
        shared: &mut Shared<R>,
        radio: &mut R,
        event: Event<R>,
    ) -> (super::super::State<R>, Result<Option<Response>, super::super::Error<'a, R>>) {
        let (new_state, result) = match self.state {
            State::Idle(state) => state.handle_event(shared, radio, event),
            State::SendingJoin(state) =>  state.handle_event(shared, radio, event),
            State::WaitingForRxWindow(state) =>state.handle_event(shared, radio, event),
            State::WaitingForJoinResponse(state) =>state.handle_event(shared, radio, event),
        };
        self.state = new_state;
        ( super::super::State::NoSession(self) , Ok(None))
    }
}


use core::default::Default;
impl<R> Default for NoSession<R>
where
    R: radio::PhyRxTx + Timings,
{
    fn default() -> Self {
        Self {
            state: State::Idle(Idle::default()),
        }
    }
}

struct Idle<R> {
    join_attempts: usize,
    radio: PhantomData<R>,
}

impl<R> Default for Idle<R>
where
    R: radio::PhyRxTx + Timings,
{
    fn default() -> Self {
        Self {
            join_attempts: 0,
            radio: PhantomData::default(),
        }
    }
}

impl<R> Idle<R>
where
    R: radio::PhyRxTx + Timings,

{
    pub fn handle_event<'a>(
        mut self,
        shared: &'a mut Shared<R>,
        radio: &mut R,
        event: Event<R>,
    ) -> (State<R>, Result<Option<Response>, super::super::Error<'a, R>>) {

        match event {
            Event::NewSession => {

                let (devnonce, tx_config) = self.create_join_request(shared);
                let radio_event: radio::Event<R> = radio::Event::TxRequest(
                    tx_config,
                    &mut shared.buffer,
                );

                // send the transmit request to the radio
                match shared.radio.handle_event(radio, radio_event) {
                    Ok(response) => {
                        match response {
                            radio::Response::Txing => {
                                (State::SendingJoin(Self::to_sending_join(self, devnonce)), Ok(None))
                            }
                            radio::Response::TxComplete => {
                                (State::WaitingForRxWindow(Self::to_waiting_rxwindow(self, devnonce)), Ok(None))
                            }
                            _ => {
                                panic ! ("Unexpected radio response: {:?}", response);
                            }
                        }
                    }
                    Err(e) => {
                        (State::Idle(self), Err(e.into()))
                    }
                }

            }
            Event::RadioEvent(radio_event) => {
                (State::Idle(self) , Ok(None))
            }
            Event::Timeout => {
                (State::Idle(self) , Ok(None))
            }
        }
    }

    fn create_join_request(&mut self, shared: &mut Shared<R>) -> (DevNonce, radio::TxConfig ) {
        let mut random = (shared.get_random)();
        // use lowest 16 bits for devnonce
        let devnonce_bytes = random as u16;

        shared.buffer.clear();
        let mut phy = JoinRequestCreator::new();
        let creds = &shared.credentials;

        let devnonce = [devnonce_bytes as u8, (devnonce_bytes >> 8) as u8];

        phy.set_app_eui(EUI64::new(creds.appeui()).unwrap())
            .set_dev_eui(EUI64::new(creds.deveui()).unwrap())
            .set_dev_nonce(&devnonce);
        let vec = phy.build(&creds.appkey()).unwrap();

        let devnonce_copy = DevNonce::new(devnonce).unwrap();
        for el in vec {
            shared.buffer.push(*el).unwrap();
        }

        // we'll use the rest for frequency and subband selection
        random >>= 16;
        let frequency = shared.region.get_join_frequency(random as u8);

        let tx_config = radio::TxConfig {
                pw: 20,
                rf: radio::RfConfig {
                    frequency,
                    bandwidth: radio::Bandwidth::_125KHZ,
                    spreading_factor: radio::SpreadingFactor::_10,
                    coding_rate: radio::CodingRate::_4_5,
                },
            };
        (devnonce_copy, tx_config)
    }

    fn to_sending_join(self, devnonce: DevNonce) -> SendingJoin<R> {
        SendingJoin {
            radio: self.radio,
            join_attempts: self.join_attempts + 1,
            devnonce,
        }
    }

    fn to_waiting_rxwindow(self, devnonce: DevNonce) -> WaitingForRxWindow<R> {
        WaitingForRxWindow {
            radio: self.radio,
            join_attempts: self.join_attempts + 1,
            devnonce,
        }
    }
}

struct SendingJoin<R> {
    join_attempts: usize,
    radio: PhantomData<R>,
    devnonce: DevNonce,
}

impl<R>  SendingJoin<R>
    where
        R: radio::PhyRxTx + Timings,
{
    pub fn handle_event<'a>(
    mut self,
    shared: &mut Shared<R>,
    radio: &mut R,
    event: Event<R>,
    ) -> (State<R>, Result<Option<Response>, super::super::Error<'a, R>>) {
        (State::SendingJoin(self) , Ok(None))
    }
}

impl<R> From<SendingJoin<R>> for WaitingForJoinResponse<R> {
    fn from(val: SendingJoin<R>) -> WaitingForJoinResponse<R> {
        WaitingForJoinResponse {
            radio: val.radio,
            devnonce: val.devnonce,
            join_attempts: val.join_attempts,
        }
    }
}

struct WaitingForRxWindow<R> {
    join_attempts: usize,
    radio: PhantomData<R>,
    devnonce: DevNonce,
}

impl<R>  WaitingForRxWindow<R>
    where
        R: radio::PhyRxTx + Timings,
{
    pub fn handle_event<'a>(
        mut self,
        shared: &mut Shared<R>,
        radio: &mut R,
        event: Event<R>,
    ) -> (State<R>, Result<Option<Response>, super::super::Error<'a, R>>) {
        (State::WaitingForRxWindow(self) , Ok(None))
    }
}

impl<R> From<WaitingForRxWindow<R>> for WaitingForJoinResponse<R> {
    fn from(val: WaitingForRxWindow<R>) -> WaitingForJoinResponse<R> {
        WaitingForJoinResponse {
            join_attempts: val.join_attempts,
            radio: val.radio,
            devnonce: val.devnonce,
        }
    }
}

struct WaitingForJoinResponse<R> {
    join_attempts: usize,
    radio: PhantomData<R>,
    devnonce: DevNonce,
}

impl<R>  WaitingForJoinResponse<R>
    where
        R: radio::PhyRxTx + Timings,
{
    pub fn handle_event<'a>(
    mut self,
    shared: &mut Shared<R>,
    radio: &mut R,
    event: Event<R>,
    ) -> (State<R>, Result<Option<Response>, super::super::Error<'a, R>>) {
        (State::WaitingForJoinResponse(self) , Ok(None))
    }
}

impl<R> From<WaitingForJoinResponse<R>> for Idle<R> {
    fn from(val: WaitingForJoinResponse<R>) -> Idle<R> {
        Idle {
            radio: val.radio,
            join_attempts: val.join_attempts,
        }
    }
}
