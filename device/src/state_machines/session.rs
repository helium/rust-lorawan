/*

This is the State Machine for a LoRaWan super-state "Session". The only way
to enter this state is for a device to be programmed in ABP mode (unimplemented)
or from a successul OTAA implemented in the NoSession module. The only way
to leave this state is to make a "Create Session" request which switches us
over to the "NoSession" super-state.

In this implementation, each state (eg: "Idle", "Txing") is a struct. When
an event is handled (eg: "SendData", "TxComplete"), a transition may or may
not occur. Regardless, a response is always given to the client, and those
are indicated here in paranthesis (ie: "(Sending)").

O
│
╔═══════════════════╗                                ╔════════════════════╗
║ Idle              ║                                ║ Txing              ║
║          SendData ║       if async       (Sending) ║                    ║
║          ─────────╫───────────────┬───────────────>║                    ║
║                   ║               │                ║         TxComplete ║
╚═══════════════════╝               │                ║          ──────────╫───┐
      ^                             │                ╚════════════════════╝   │
      │                             │                                         │
      │                             │                                         │
┌─────┘    ╔═══════════════════╗    │          ╔════════════════════╗         │
│          ║ WaitingForRx      ║    │          ║ WaitingForRxWindow ║         │
│          ║ ╔═════════════╗   ║    │else sync ║  ╔═════════════╗   ║         │
│          ║ ║ RxWindow1   ║   ║    └──────────╫─>║ RxWindow1   ║<──╫─────────┘
│(DataDown)║ ║    Rx       ║   ║   (TimeoutReq)║  ║             ║   ║(TimeoutReq)
├──────────╫─╫───────      ║   ║(TimeoutReq)   ║  ║    Timeout  ║   ║
│          ║ ║    Timeout  ║<──╫───────────────╫──╫──────────── ║   ║
│          ║ ║    ─────────╫───╫──┐            ║  ╚═════════════╝   ║
│          ║ ╚═════════════╝   ║  │            ║                    ║
│          ║ ╔═════════════╗   ║  │(TimeoutReq)║   ╔═════════════╗  ║
│(DataDown)║ ║ RxWindow2   ║   ║  └────────────╫─> ║ RxWindow2   ║  ║
├──────────╫─╫──┐ Rx       ║   ║               ║   ║             ║  ║
│          ║ ║  └───       ║   ║(TimeoutReq)   ║   ║    Timeout  ║  ║
│ if conf  ║ ║    Timeout  ║<──╫───────────────╫───╫──────────── ║  ║
│ (NoACK)  ║ ║   ┌──────── ║   ║               ║   ╚═════════════╝  ║
└──────────╫─╫───┘         ║   ║               ║                    ║
else(Ready)║ ╚═════════════╝   ║               ║                    ║
           ╚═══════════════════╝               ╚════════════════════╝
 */

use super::super::no_session::SessionData;
use super::super::State as SuperState;
use super::super::*;
use as_slice::AsSlice;
use core::marker::PhantomData;
use lorawan_encoding::{
    self,
    creator::DataPayloadCreator,
    keys::AES128,
    maccommands::SerializableMacCommand,
    parser::{parse as lorawan_parse, *},
};

pub enum Session<R>
where
    R: radio::PhyRxTx + Timings,
{
    Idle(Idle<R>),
    SendingData(SendingData<R>),
    WaitingForRxWindow(WaitingForRxWindow<R>),
    WaitingForRx(WaitingForRx<R>),
}

enum RxWindow {
    _1(u32),
    _2(u32),
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

into_state![Idle, SendingData, WaitingForRxWindow, WaitingForRx];

pub enum Error {}

impl<R> Session<R>
where
    R: radio::PhyRxTx + Timings,
{
    pub fn new(shared: Shared<R>, session: SessionData) -> Device<R> {
        Device {
            state: SuperState::Session(Session::Idle(Idle { shared, session })),
        }
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
            .set_fcnt(self.session.fcnt_up());

        let mut cmds = Vec::new();
        self.shared.mac.get_cmds(&mut cmds);

        let mut dyn_cmds: Vec<&dyn SerializableMacCommand, U8> = Vec::new();

        for cmd in &cmds {
            if let Err(_e) = dyn_cmds.push(cmd) {
                panic!("dyn_cmds too small compared to cmds")
            }
        }

        self.session.fcnt_up_increment();

        match phy.build(
            &data.data,
            dyn_cmds.as_slice(),
            self.session.newskey(),
            self.session.appskey(),
        ) {
            Ok(packet) => {
                self.shared.buffer.clear();
                self.shared.buffer.extend(packet);
            }
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
                            radio::Response::Txing => (
                                self.to_sending_data(confirmed).into(),
                                Ok(Response::SendingDataUp),
                            ),
                            // directly jump to waiting for RxWindow
                            // allows for synchronous sending
                            radio::Response::TxComplete(ms) => {
                                data_rxwindow1_timeout(Session::Idle(self), confirmed, ms)
                            }
                            _ => {
                                panic!("Idle: Unexpected radio response: {:?}", response);
                            }
                        }
                    }
                    Err(e) => {
                        panic!("Radio Error when Idle: {:?}", e)
                        //({self.into(), Err(e.into())),
                        }
                    }
            }
            // tolerate unexpected timeout
            Event::Timeout => (self.into(), Ok(Response::Idle)),
            Event::NewSession => {
                panic!("Unhandled NewSession request during active session");
            },
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

    fn into_waiting_for_rxwindow(self, confirmed: bool, time: u32) -> WaitingForRxWindow<R> {
        WaitingForRxWindow {
            session: self.session,
            shared: self.shared,
            rx_window: RxWindow::_1(time),
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

impl<R> SendingData<R>
where
    R: radio::PhyRxTx + Timings,
{
    pub fn handle_event<'a>(
        mut self,
        radio: &'a mut R,
        event: Event<R>,
    ) -> (Device<R>, Result<Response, super::super::Error>) {
        match event {
            // we are waiting for the async tx to complete
            Event::RadioEvent(radio_event) => {
                // send the transmit request to the radio
                match self.shared.radio.handle_event(radio, radio_event) {
                    Ok(response) => {
                        match response {
                            // expect a complete transmit
                            radio::Response::TxComplete(ms) => {
                                let confirmed = self.confirmed;
                                data_rxwindow1_timeout(Session::SendingData(self), confirmed,ms)
                            }
                            // anything other than TxComplete is unexpected
                            _ => {
                                panic!("SendingData: Unexpected radio response: {:?}", response);
                            }
                        }
                    }
                    Err(e) => {
                        panic!("Radio Error when SendingData: {:?}", e)
                    },
                }
            }
            // tolerate unexpected timeout
            Event::Timeout => (self.into(), Ok(Response::Idle)),
            // anything other than a RadioEvent is unexpected
            Event::NewSession | Event::Timeout | Event::SendData(_) => {
                panic!("Unexpected event while SendingJoin")
            }
        }
    }

    fn into_waiting_for_rxwindow(self, confirmed: bool, time: u32) -> WaitingForRxWindow<R> {
        WaitingForRxWindow {
            session: self.session,
            shared: self.shared,
            rx_window: RxWindow::_1(time),
            confirmed,
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
    rx_window: RxWindow,
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
            // we are waiting for a Timeout
            Event::Timeout => {
                let rx_config = radio::RfConfig {
                    frequency: self.shared.region.get_rxwindow1_frequency(),
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
                    Ok(_) => {

                        let window_close: u32 = match self.rx_window {
                            // RxWindow1 one must timeout before RxWindow2
                            RxWindow::_1(time) => {
                                let time_between_windows = self.shared.region.get_receive_delay2() -
                                    self.shared.region.get_receive_delay1();
                                if time_between_windows > radio.get_rx_window_duration_ms() {
                                    time + radio.get_rx_window_duration_ms()
                                } else {
                                    time + time_between_windows
                                }
                            },
                            // RxWindow2 can last however long
                            RxWindow::_2(time) => time + radio.get_rx_window_duration_ms(),
                        };
                        (
                            WaitingForRx::from(self).into(),
                            Ok(Response::TimeoutRequest(window_close)),
                        )
                    }
                    Err(e) => {
                        panic!("Radio Error when WaitingForRxWindow: {:?}", e)
                        //(self.into(), Err(e.into()))
                    },
                }
            }
            // anything other than a Timeout is unexpected
            Event::NewSession | Event::RadioEvent(_) | Event::SendData(_) => {
                panic!("Unexpected event while WaitingForRxWindow")
            }
        }
    }
}

impl<R> From<WaitingForRxWindow<R>> for WaitingForRx<R>
where
    R: radio::PhyRxTx + Timings,
{
    fn from(val: WaitingForRxWindow<R>) -> WaitingForRx<R> {
        WaitingForRx {
            shared: val.shared,
            session: val.session,
            confirmed: val.confirmed,
            rx_window: val.rx_window,
        }
    }
}

pub struct WaitingForRx<R>
where
    R: radio::PhyRxTx + Timings,
{
    shared: Shared<R>,
    session: SessionData,
    confirmed: bool,
    rx_window: RxWindow,
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
            // we are waiting for the async tx to complete
            Event::RadioEvent(radio_event) => {
                // send the transmit request to the radio
                match self.shared.radio.handle_event(radio, radio_event) {
                    Ok(response) => match response {
                        radio::Response::Rx(quality) => {
                            let packet = lorawan_parse(radio.get_received_packet()).unwrap();
                            if let PhyPayload::Data(data_frame) = packet {
                                if let DataPayload::Encrypted(encrypted_data) = data_frame {
                                    let session = &mut self.session;
                                    if session.devaddr() == &encrypted_data.fhdr().dev_addr() {
                                        let fcnt = encrypted_data.fhdr().fcnt() as u32;
                                        if encrypted_data.validate_mic(&session.newskey(), fcnt)
                                            && (fcnt > session.fcnt_down || fcnt == 0)
                                        {
                                            session.fcnt_down = fcnt;
                                            let decrypted = encrypted_data
                                                .decrypt(
                                                    Some(&session.newskey()),
                                                    Some(&session.appskey()),
                                                    fcnt,
                                                )
                                                .unwrap();

                                            for mac_cmd in decrypted.fhdr().fopts() {
                                                self.shared.mac.handle_downlink_mac(
                                                    &mut self.shared.region,
                                                    &mac_cmd,
                                                );
                                            }
                                            return (self.into_idle().into(), Ok(Response::DataDown));
                                        }
                                    }
                                }
                            }
                            (self.into(), Ok(Response::WaitingForDataDown))
                        }
                        _ => (self.into(), Ok(Response::WaitingForDataDown)),
                    },
                    Err(e) => (self.into(), Err(e.into())),
                }
            }
            Event::Timeout => {

                // send the transmit request to the radio
                if let Err(e) = self.shared.radio.handle_event(radio, radio::Event::CancelRx) {
                    panic!("Error cancelling Rx: {:?}", e)
                }

                match self.rx_window {
                    RxWindow::_1(t1) => {
                        let time_between_windows = self.shared.region.get_receive_delay2() -
                            self.shared.region.get_receive_delay1();
                        let t2 = t1+ time_between_windows;
                        // TODO: jump to RxWindow2 if t2 == now
                        (WaitingForRxWindow {
                            shared: self.shared,
                            session: self.session,
                            confirmed: self.confirmed,
                            rx_window: RxWindow::_2(t2)
                        }.into(), Ok(Response::TimeoutRequest(t2)))
                    }
                    // Timeout during second RxWindow leads to giving up
                    RxWindow::_2(_) => {
                        (Idle {
                            shared: self.shared,
                            session: self.session,
                        }.into(), match self.confirmed {
                            true => Ok(Response::NoAck),
                            false => Ok(Response::ReadyToSend),
                        })
                    }
                }
            }
            // anything other than a RadioEvent is unexpected
            Event::NewSession | Event::SendData(_) => {
                panic!("Unexpected event while WaitingForRx")
            }
        }
    }

    fn into_idle(self) -> Idle<R> {
        Idle {
            shared: self.shared,
            session: self.session,
        }
    }

}

fn data_rxwindow1_timeout<R: radio::PhyRxTx + Timings>(state: Session<R>, confirmed: bool, timestamp_ms: TimestampMs) -> (Device<R>, Result<Response, super::super::Error>) {

    let (new_state, first_window) = match state {
        Session::Idle(state) => {
            let first_window = state.shared.region.get_receive_delay1() + timestamp_ms;
            (state.into_waiting_for_rxwindow(confirmed,first_window), first_window)
        },
        Session::SendingData(state) => {
            let first_window = state.shared.region.get_receive_delay1() + timestamp_ms;
            (state.into_waiting_for_rxwindow(confirmed,first_window), first_window)

        },
        _ => panic!("Invalid state to transition to WaitingForRxWindow")
    };

    (
        new_state.into(),
        Ok(Response::TimeoutRequest(first_window)),
    )
}
