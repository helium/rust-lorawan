use super::*;
use core::marker::PhantomData;

pub mod no_session;
use no_session::NoSession;

pub mod session;
use session::Session;

pub enum State<R>
where
    R: radio::PhyRxTx + Timings,
{
    NoSession(NoSession<R>),
    Session(Session<R>),
}
