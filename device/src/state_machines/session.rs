use super::super::State as SuperState;
use super::super::*;
use super::super::no_session::SessionData;
use core::marker::PhantomData;
use lorawan_encoding::{
    self,
    keys::AES128,
    creator::DataPayloadCreator,
    maccommands::SerializableMacCommand,
    parser::{parse as lorawan_parse, *},
};
use as_slice::AsSlice;


pub enum Session<R>
where
    R: radio::PhyRxTx + Timings,
{
    Idle(Idle<R>),
    SendingData(SendingData<R>),
    WaitingForRxWindow(WaitingForRxWindow<R>),
    WaitingForRx(WaitingForRx<R>),
}


macro_rules! into_state {
    ($($from:tt),*) => {
    $(
        impl<R> From<$from<R>> for Device<R>
        where
            R: radio::PhyRxTx + Timings,
        {
            fn from(state: $from<R>) -> Device<R> {
                Device { state: SuperState::Session(Session::$from(state)) }
            }
        }
    )*};
}

into_state![
    Idle,
    SendingData,
    WaitingForRxWindow,
    WaitingForRx
];

pub enum Error {}

impl<R> Session<R>
    where
        R: radio::PhyRxTx + Timings
{
    pub fn new(shared: Shared<R>, session: SessionData) -> Device<R>{
        Device{ state: SuperState::Session(Session::Idle( Idle {
            shared,
            session,
        }))}
    }
    pub fn handle_event(
        mut self,
        radio: &mut R,
        event: Event<R>,
    ) -> (Device<R>, Result<Response, super::super::Error>) {
        match self {
            Session::Idle(state) => state.handle_event(radio, event),
            Session::SendingData(state) => state.handle_event(radio, event),
            Session::WaitingForRxWindow(state) => state.handle_event(radio, event),
            Session::WaitingForRx(state) => state.handle_event(radio, event),
        }
    }
}

impl<'a, R> Idle<R>
    where
        R: radio::PhyRxTx + Timings,
{

    fn prepare_buffer(&mut self, data: &SendData) {

        let mut phy = DataPayloadCreator::new();
        phy.set_confirmed(data.confirmed)
            .set_f_port(data.fport)
            .set_dev_addr(self.session.devaddr().clone())
            .set_fcnt(self.session.fcnt());

        let mut cmds = Vec::new();
        self.shared.mac.get_cmds(&mut cmds);

        let mut dyn_cmds: Vec<&dyn SerializableMacCommand, U8> = Vec::new();

        for cmd in &cmds {
            if let Err(_e) = dyn_cmds.push(cmd) {
                panic!("dyn_cmds too small compared to cmds")
            }
        }

        match phy.build(
            &data.data,
            dyn_cmds.as_slice(),
            self.session.newskey(),
            self.session.appskey(),
        ) {
            Ok(packet) => {
                self.shared.buffer.clear();
                self.shared.buffer.extend(packet);
            },
            Err(_) => panic!("Error assembling packet!"),
        }
    }
    pub fn handle_event(
        mut self,
        radio: &'a mut R,
        event: Event<R>,
    ) -> (Device<R>, Result<Response, super::super::Error>) {
        match event {
            Event::SendData(send_data) => {

                // encodes the packet and places it in send buffer
                self.prepare_buffer(&send_data);

                let mut random = (self.shared.get_random)();
                let frequency = self.shared.region.get_join_frequency(random as u8);

                let event: radio::Event<R> = radio::Event::TxRequest(
                    radio::TxConfig {
                        pw: 20,
                        rf: radio::RfConfig {
                            frequency,
                            bandwidth: radio::Bandwidth::_125KHZ,
                            spreading_factor: radio::SpreadingFactor::_10,
                            coding_rate: radio::CodingRate::_4_5,
                        },
                    },
                    &mut self.shared.buffer,
                );

                let confirmed = send_data.confirmed;

                // send the transmit request to the radio
                match self.shared.radio.handle_event(radio, event) {
                    Ok(response) => {
                        match response {
                            // intermediate state where we wait for Join to complete sending
                            // allows for asynchronous sending
                            radio::Response::Txing => {
                                (self.to_sending_data(confirmed).into(), Ok(Response::SendingDataUp))
                            }
                            // directly jump to waiting for RxWindow
                            // allows for synchronous sending
                            radio::Response::TxComplete(ms) => {
                                let time = data_rx_window_timeout(&self.shared.region, ms);
                                (
                                    self.to_waiting_rxwindow(confirmed).into(),
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
            Event::NewSession | Event::Timeout => {
                (self.into(), Ok(Response::Idle))
            }
            Event::RadioEvent(radio_event) => {
                panic!("Unexpected radio event while Session::Idle");
            }
        }
    }

    fn to_sending_data(self, confirmed: bool) -> SendingData<R> {
        SendingData {
            session: self.session,
            shared: self.shared,
            confirmed,
        }
    }

    fn to_waiting_rxwindow(self, confirmed: bool)  -> WaitingForRxWindow<R> {
        WaitingForRxWindow {
            session: self.session,
            shared: self.shared,
            confirmed,
        }
    }
}

pub struct Idle<R>
where
    R: radio::PhyRxTx + Timings,
{
    shared: Shared<R>,
    session: SessionData,
}

pub struct SendingData<R>
where
    R: radio::PhyRxTx + Timings,
{
    shared: Shared<R>,
    session: SessionData,
    confirmed: bool,
}

impl<'a, R> SendingData<R>
    where
        R: radio::PhyRxTx + Timings,
{
    pub fn handle_event(
        mut self,
        radio: &'a mut R,
        event: Event<R>,
    ) -> (Device<R>, Result<Response, super::super::Error>) {
        match event {
            Event::SendData(_) => {
                (self.into(), Ok(Response::Idle))
            }
            Event::NewSession | Event::Timeout => {
                (self.into(), Ok(Response::Idle))
            }
            Event::RadioEvent(radio_event) => {
                panic!("Unexpected radio event while Session::Idle");
            }
        }
    }
}

pub struct WaitingForRxWindow<R>
    where
        R: radio::PhyRxTx + Timings,
{
    shared: Shared<R>,
    session: SessionData,
    confirmed: bool,
}

impl<'a, R> WaitingForRxWindow<R>
    where
        R: radio::PhyRxTx + Timings,
{
    pub fn handle_event(
        mut self,
        radio: &'a mut R,
        event: Event<R>,
    ) -> (Device<R>, Result<Response, super::super::Error>) {
        match event {
            Event::SendData(_) => {
                (self.into(), Ok(Response::Idle))
            }
            Event::NewSession | Event::Timeout => {
                (self.into(), Ok(Response::Idle))
            }
            Event::RadioEvent(radio_event) => {
                panic!("Unexpected radio event while Session::Idle");
            }
        }
    }
}


pub struct WaitingForRx<R>
where
    R: radio::PhyRxTx + Timings,
{
    shared: Shared<R>,
    session: SessionData,
    radio: PhantomData<R>,
}

impl<'a, R> WaitingForRx<R>
    where
        R: radio::PhyRxTx + Timings,
{
    pub fn handle_event(
        mut self,
        radio: &'a mut R,
        event: Event<R>,
    ) -> (Device<R>, Result<Response, super::super::Error>) {
        match event {
            Event::SendData(_) => {
                (self.into(), Ok(Response::Idle))
            }
            Event::NewSession | Event::Timeout => {
                (self.into(), Ok(Response::Idle))
            }
            Event::RadioEvent(radio_event) => {
                panic!("Unexpected radio event while Session::Idle");
            }
        }
    }
}

fn data_rx_window_timeout(region: &RegionalConfiguration, timestamp_ms: TimestampMs) -> u32 {
    region.get_receive_delay1() + timestamp_ms
}
