use super::super::*;
use core::marker::PhantomData;

pub enum State<R>
where
    R: radio::PhyRxTx + Timings,
{
    Idle(Idle<R>),
    SendingData(SendingData<R>),
    WaitingForRx(WaitingForRx<R>),
}

pub enum Error {}

pub struct Session<R>
where
    R: radio::PhyRxTx + Timings,
{
    state: State<R>,
}

impl<R> Session<R>
    where
        R: radio::PhyRxTx + Timings
{
    pub fn handle_event<'a>(
        mut self,
        shared: &mut Shared<R>,
        radio: &mut R,
        event: Event<R>,
    ) -> (super::super::State<R>, Result<Option<Response>, super::super::Error<'a, R>>) {
        ( super::super::State::Session(self) , Ok(None))
    }
}

struct Idle<R>
where
    R: radio::PhyRxTx + Timings,
{
    radio: PhantomData<R>,
}

struct SendingData<R>
where
    R: radio::PhyRxTx + Timings,
{
    radio: PhantomData<R>,
}

struct WaitingForRx<R>
where
    R: radio::PhyRxTx + Timings,
{
    radio: PhantomData<R>,
}
