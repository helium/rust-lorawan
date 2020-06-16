use super::super::State as SuperState;
use super::super::*;
use super::super::no_session::SessionData;
use core::marker::PhantomData;
use lorawan_encoding::{
    self,
    keys::AES128,
    parser::{parse as lorawan_parse, *},
};

pub enum Session<R>
where
    R: radio::PhyRxTx + Timings,
{
    Idle(Idle<R>),
    SendingData(SendingData<R>),
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
            radio: PhantomData::default()
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
            Session::WaitingForRx(state) => state.handle_event(radio, event),
        }
    }
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
        (self.into(), Ok(Response::Idle))
    }
}

pub struct Idle<R>
where
    R: radio::PhyRxTx + Timings,
{
    shared: Shared<R>,
    session: SessionData,
    radio: PhantomData<R>,
}

pub struct SendingData<R>
where
    R: radio::PhyRxTx + Timings,
{
    shared: Shared<R>,
    session: SessionData,
    radio: PhantomData<R>,
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
        (self.into(), Ok(Response::Idle))
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
        (self.into(), Ok(Response::Idle))
    }
}
