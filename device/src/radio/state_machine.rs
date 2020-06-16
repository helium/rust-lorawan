use super::*;
use core::marker::PhantomData;

#[derive(Copy, Clone)]
pub enum State<R>
where
    R: PhyRxTx,
{
    Idle(Idle<R>),
    Txing(Txing<R>),
    Rxing(Rxing<R>),
}

#[derive(Debug)]
pub enum Response {
    TxComplete(TimestampMs), // packet sent
    Txing,                   // sending packet
    Rx(RxQuality),           // packet received
    Rxing,                   // in receiving mode
    Idle,
}

pub enum Error {
    BadState,
    PhyError(PhyError),
}

use core::convert::From;

impl From<Error> for super::super::Error {
    fn from(error: Error) -> Self {
        super::super::Error::RadioError(error)
    }
}

#[derive(Debug)]
pub enum Event<'a, R>
where
    R: PhyRxTx,
{
    TxRequest(TxConfig, &'a mut Vec<u8, U256>),
    RxRequest(RfConfig),
    PhyEvent(R::PhyEvent),
    Timeout,
}

#[derive(Copy, Clone)]
pub struct StateWrapper<R>
where
    R: PhyRxTx,
{
    radio_state: State<R>,
}

use core::default::Default;

impl<R> Default for StateWrapper<R>
where
    R: PhyRxTx,
{
    fn default() -> Self {
        Self {
            radio_state: State::Idle(Idle {
                radio: PhantomData::default(),
            }),
        }
    }
}

impl<R> StateWrapper<R>
where
    R: PhyRxTx,
{
    pub fn handle_event(&mut self, radio: &mut R, event: Event<R>) -> Result<Response, Error> {
        let (new_state, response) = match &self.radio_state {
            State::Idle(state) => state.handle_event(radio, event),
            State::Txing(state) => state.handle_event(radio, event),
            State::Rxing(state) => state.handle_event(radio, event),
        };
        self.radio_state = new_state;
        response
    }
}

macro_rules! default_transition {
    ($from:tt,$to:tt) => {
        impl<R> From<$from<R>> for $to<R> {
            fn from(val: $from<R>) -> $to<R> {
                $to { radio: val.radio }
            }
        }
    };
}

macro_rules! state {
    (
        $name:tt; [ $( $y:tt ),* ]
       ) => {
        pub struct $name<R> {
            radio: PhantomData<R>,
        }

        $(default_transition![
          $name, $y
        ];)*

        impl<R> Clone for $name<R> {
            fn clone(&self) -> Self {
                Self {
                    radio: PhantomData::default()
                }
            }
        }

        impl<R> Copy for $name<R> {}
    };
}

state![Idle; [Txing, Rxing]];

impl<R> Idle<R>
where
    R: PhyRxTx,
{
    fn handle_event(
        mut self,
        radio: &mut R,
        event: Event<R>,
    ) -> (State<R>, Result<Response, Error>) {
        match event {
            Event::TxRequest(config, buf) => {
                radio.configure_tx(config);
                radio.send(buf.as_mut());
                (State::Txing(self.into()), Ok(Response::Txing))
            }
            Event::RxRequest(rfconfig) => {
                radio.configure_rx(rfconfig);
                radio.set_rx();
                (State::Rxing(self.into()), Ok(Response::Rxing))
            }
            _ => (State::Idle(self), Err(Error::BadState)),
        }
    }
}

state![Txing; [Idle]];
impl<R> Txing<R>
where
    R: PhyRxTx,
{
    fn handle_event(
        mut self,
        radio: &mut R,
        event: Event<R>,
    ) -> (State<R>, Result<Response, Error>) {
        match event {
            Event::PhyEvent(phyevent) => {
                if let Some(PhyResponse::TxDone(timestamp_ms)) = radio.handle_phy_event(phyevent) {
                    (
                        State::Idle(self.into()),
                        Ok(Response::TxComplete(timestamp_ms)),
                    )
                } else {
                    (State::Txing(self), Ok(Response::Txing))
                }
            }
            Event::TxRequest(_, _) => (State::Txing(self), Err(Error::BadState)),
            Event::RxRequest(_) => (State::Txing(self), Err(Error::BadState)),
            Event::Timeout => {
                if let Err(e) = radio.cancel_tx() {
                    (State::Idle(self.into()), Err(Error::PhyError(e)))
                } else {
                    (State::Idle(self.into()), Ok(Response::Idle))
                }
            }
        }
    }
}

state![Rxing; [Idle]];
impl<R> Rxing<R>
where
    R: PhyRxTx,
{
    fn handle_event<'a>(
        mut self,
        radio: &mut R,
        event: Event<'a, R>,
    ) -> (State<R>, Result<Response, Error>) {
        match event {
            Event::PhyEvent(phyevent) => {
                if let Some(PhyResponse::RxDone(quality)) = radio.handle_phy_event(phyevent) {
                    (State::Idle(self.into()), Ok(Response::Rx(quality)))
                } else {
                    (State::Rxing(self), Ok(Response::Rxing))
                }
            }
            Event::TxRequest(_, _) => (State::Rxing(self), Err(Error::BadState)),
            Event::RxRequest(_) => (State::Rxing(self), Err(Error::BadState)),
            Event::Timeout => {
                if let Err(e) = radio.cancel_rx() {
                    (State::Idle(self.into()), Err(Error::PhyError(e)))
                } else {
                    (State::Idle(self.into()), Ok(Response::Idle))
                }
            }
        }
    }
}
