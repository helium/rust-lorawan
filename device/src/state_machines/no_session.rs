use super::super::State as SuperState;
use super::super::*;
use super::Shared;
use core::marker::PhantomData;
use lorawan_encoding::{
    self,
    creator::JoinRequestCreator,
    keys::AES128,
    parser::DevAddr,
    parser::{parse as lorawan_parse, *},
};

pub enum NoSession<R>
where
    R: radio::PhyRxTx + Timings,
{
    Idle(Idle<R>),
    SendingJoin(SendingJoin<R>),
    WaitingForRxWindow(WaitingForRxWindow<R>),
    WaitingForJoinResponse(WaitingForJoinResponse<R>),
}

macro_rules! into_state {
    ($($from:tt),*) => {
    $(
        impl<R> From<$from<R>> for Device<R>
        where
            R: radio::PhyRxTx + Timings,
        {
            fn from(state: $from<R>) -> Device<R> {
                Device { state: SuperState::NoSession(NoSession::$from(state)) }
            }
        }
    )*};
}

into_state![
    Idle,
    SendingJoin,
    WaitingForRxWindow,
    WaitingForJoinResponse
];

impl<R> From<NoSession<R>> for SuperState<R>
where
    R: radio::PhyRxTx + Timings,
{
    fn from(no_session: NoSession<R>) -> SuperState<R> {
        SuperState::NoSession(no_session)
    }
}

impl<R> NoSession<R>
where
    R: radio::PhyRxTx + Timings,
{
    pub fn new(shared: Shared<R>) -> NoSession<R> {
        NoSession::Idle(Idle {
            shared,
            join_attempts: 0,
            radio: PhantomData::default(),
        })
    }

    pub fn handle_event(
        mut self,
        radio: &mut R,
        event: Event<R>,
    ) -> (Device<R>, Result<Response, super::super::Error>) {
        match self {
            NoSession::Idle(state) => state.handle_event(radio, event),
            NoSession::SendingJoin(state) => state.handle_event(radio, event),
            NoSession::WaitingForRxWindow(state) => state.handle_event(radio, event),
            NoSession::WaitingForJoinResponse(state) => state.handle_event(radio, event),
        }
    }
}

pub enum Error {}

type DevNonce = lorawan_encoding::parser::DevNonce<[u8; 2]>;

pub struct Idle<R>
where
    R: radio::PhyRxTx + Timings,
{
    shared: Shared<R>,
    join_attempts: usize,
    radio: PhantomData<R>,
}

impl<'a, R> Idle<R>
where
    R: radio::PhyRxTx + Timings,
{
    pub fn handle_event(
        mut self,
        radio: &'a mut R,
        event: Event<R>,
    ) -> (Device<R>, Result<Response, super::super::Error>) {
        match event {
            // NewSession Request or a Timeout from previously failed Join attempt
            Event::NewSession | Event::Timeout => {
                let (devnonce, tx_config) = self.create_join_request();
                let radio_event: radio::Event<R> =
                    radio::Event::TxRequest(tx_config, &mut self.shared.buffer);

                // send the transmit request to the radio
                match self.shared.radio.handle_event(radio, radio_event) {
                    Ok(response) => {
                        match response {
                            // intermediate state where we wait for Join to complete sending
                            // allows for asynchronous sending
                            radio::Response::Txing => {
                                (self.to_sending_join(devnonce).into(), Ok(Response::Idle))
                            }
                            // directly jump to waiting for RxWindow
                            // allows for synchronous sending
                            radio::Response::TxComplete(ms) => {
                                let time = join_rx_window_timeout(&self.shared.region, ms);

                                (
                                    self.to_waiting_rxwindow(devnonce).into(),
                                    Ok(Response::TimeoutRequest(time)),
                                )
                            }
                            _ => {
                                panic!("Unexpected radio response: {:?}", response);
                            }
                        }
                    }
                    Err(e) => (self.into(), Err(e.into())),
                }
            }
            Event::RadioEvent(radio_event) => {
                panic!("Unexpected radio event while Idle");
            }
        }
    }

    fn create_join_request(&mut self) -> (DevNonce, radio::TxConfig) {
        let mut random = (self.shared.get_random)();
        // use lowest 16 bits for devnonce
        let devnonce_bytes = random as u16;

        self.shared.buffer.clear();
        let mut phy = JoinRequestCreator::new();
        let creds = &self.shared.credentials;

        let devnonce = [devnonce_bytes as u8, (devnonce_bytes >> 8) as u8];

        phy.set_app_eui(EUI64::new(creds.appeui()).unwrap())
            .set_dev_eui(EUI64::new(creds.deveui()).unwrap())
            .set_dev_nonce(&devnonce);
        let vec = phy.build(&creds.appkey()).unwrap();

        let devnonce_copy = DevNonce::new(devnonce).unwrap();
        for el in vec {
            self.shared.buffer.push(*el).unwrap();
        }

        // we'll use the rest for frequency and subband selection
        random >>= 16;
        let frequency = self.shared.region.get_join_frequency(random as u8);

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
            shared: self.shared,
            radio: self.radio,
            join_attempts: self.join_attempts + 1,
            devnonce,
        }
    }

    fn to_waiting_rxwindow(self, devnonce: DevNonce) -> WaitingForRxWindow<R> {
        WaitingForRxWindow {
            shared: self.shared,
            radio: self.radio,
            join_attempts: self.join_attempts + 1,
            devnonce,
        }
    }
}

pub struct SendingJoin<R>
where
    R: radio::PhyRxTx + Timings,
{
    shared: Shared<R>,
    join_attempts: usize,
    radio: PhantomData<R>,
    devnonce: DevNonce,
}

impl<R> SendingJoin<R>
where
    R: radio::PhyRxTx + Timings,
{
    pub fn handle_event(
        mut self,
        radio: &mut R,
        event: Event<R>,
    ) -> (Device<R>, Result<Response, super::super::Error>) {
        match event {
            // we are waiting for the async tx to complete
            Event::RadioEvent(radio_event) => {
                // send the transmit request to the radio
                match self.shared.radio.handle_event(radio, radio_event) {
                    Ok(response) => {
                        match response {
                            radio::Response::TxComplete(ms) => {
                                let time = join_rx_window_timeout(&self.shared.region, ms);
                                (self.into(), Ok(Response::TimeoutRequest(time)))
                            }
                            // anything other than TxComplete is unexpected
                            _ => {
                                panic!("Unexpected radio response: {:?}", response);
                            }
                        }
                    }
                    Err(e) => (self.into(), Err(e.into())),
                }
            }
            // anything other than a RadioEvent is unexpected
            Event::NewSession | Event::Timeout => panic!("Unexpected event while SendingJoin"),
        }
    }
}

impl<R> From<SendingJoin<R>> for WaitingForRxWindow<R>
where
    R: radio::PhyRxTx + Timings,
{
    fn from(val: SendingJoin<R>) -> WaitingForRxWindow<R> {
        WaitingForRxWindow {
            shared: val.shared,
            join_attempts: val.join_attempts,
            radio: val.radio,
            devnonce: val.devnonce,
        }
    }
}

pub struct WaitingForRxWindow<R>
where
    R: radio::PhyRxTx + Timings,
{
    shared: Shared<R>,
    join_attempts: usize,
    radio: PhantomData<R>,
    devnonce: DevNonce,
}

impl<R> WaitingForRxWindow<R>
where
    R: radio::PhyRxTx + Timings,
{
    pub fn handle_event<'a>(
        mut self,
        radio: &mut R,
        event: Event<R>,
    ) -> (Device<R>, Result<Response, super::super::Error>) {
        match event {
            // we are waiting for a Timeout
            Event::Timeout => {
                let rx_config = radio::RfConfig {
                    frequency: self.shared.region.get_join_accept_frequency1(),
                    bandwidth: radio::Bandwidth::_500KHZ,
                    spreading_factor: radio::SpreadingFactor::_10,
                    coding_rate: radio::CodingRate::_4_5,
                };
                // configure the radio for the RX
                match self
                    .shared
                    .radio
                    .handle_event(radio, radio::Event::RxRequest(rx_config))
                {
                    // TODO: pass timeout
                    Ok(_) => (self.into(), Ok(Response::Idle)),
                    Err(e) => (self.into(), Err(e.into())),
                }
            }
            // anything other than a Timeout is unexpected
            Event::NewSession | Event::RadioEvent(_) => {
                panic!("Unexpected event while WaitingForRxWindow")
            }
        }
    }
}

impl<R> From<WaitingForRxWindow<R>> for WaitingForJoinResponse<R>
where
    R: radio::PhyRxTx + Timings,
{
    fn from(val: WaitingForRxWindow<R>) -> WaitingForJoinResponse<R> {
        WaitingForJoinResponse {
            shared: val.shared,
            radio: val.radio,
            join_attempts: val.join_attempts,
            devnonce: val.devnonce,
        }
    }
}

pub struct WaitingForJoinResponse<R>
where
    R: radio::PhyRxTx + Timings,
{
    shared: Shared<R>,
    join_attempts: usize,
    radio: PhantomData<R>,
    devnonce: DevNonce,
}

impl<R> WaitingForJoinResponse<R>
where
    R: radio::PhyRxTx + Timings,
{
    pub fn handle_event<'a>(
        self,
        radio: &mut R,
        event: Event<R>,
    ) -> (Device<R>, Result<Response, super::super::Error>) {
        (self.into(), Ok(Response::Idle))
    }
}

impl<R> From<WaitingForJoinResponse<R>> for Idle<R>
where
    R: radio::PhyRxTx + Timings,
{
    fn from(val: WaitingForJoinResponse<R>) -> Idle<R> {
        Idle {
            shared: val.shared,
            radio: val.radio,
            join_attempts: val.join_attempts,
        }
    }
}

fn join_rx_window_timeout(region: &RegionalConfiguration, timestamp_ms: TimestampMs) -> u32 {
    region.get_join_accept_delay1() + timestamp_ms
}
