use lorawan_encoding::maccommands::ChannelMask;

mod us915;
mod cn470;
mod eu868;

pub use us915::US915;
pub use cn470::CN470;
pub use eu868::EU868;

pub struct Configuration {
    state: State,
}

#[derive(Debug, Clone)]
pub enum Region {
    US915,
    CN470,
    EU868
}

enum State {
    US915(US915),
    CN470(CN470),
    EU868(EU868),
}

impl State {
    pub fn new(region: Region) -> State {
        match region {
            Region::US915 => State::US915(US915::new()),
            Region::CN470 => State::CN470(CN470::new()),
            Region::EU868 => State::EU868(EU868::new()),
        }
    }
}

impl Configuration {
    pub fn new(region: Region) -> Configuration {
        Configuration {
            state: State::new(region),
        }
    }
}

use lorawan_encoding::parser::DecryptedJoinAcceptPayload;

impl RegionHandler for Configuration {
    fn process_join_accept<T: core::convert::AsRef<[u8]>,C>(&mut self, join_accept: &DecryptedJoinAcceptPayload<T, C>){
        match & mut self.state {
            State::US915(state) => state.process_join_accept(join_accept),
            State::CN470(state) => state.process_join_accept(join_accept),
            State::EU868(state) => state.process_join_accept(join_accept),
        }
    }

    fn set_channel_mask(&mut self, channel_mask: ChannelMask) {
        match &mut self.state {
            State::US915(state) => state.set_channel_mask(channel_mask),
            State::CN470(state) => state.set_channel_mask(channel_mask),
            State::EU868(state) => state.set_channel_mask(channel_mask),
        }
    }

    fn set_subband(&mut self, subband: u8) {
        match &mut self.state {
            State::US915(state) => state.set_subband(subband),
            State::CN470(state) => state.set_subband(subband),
            State::EU868(state) => state.set_subband(subband),

        }
    }
    fn get_join_frequency(&mut self, random: u8) -> u32 {
        match &mut self.state {
            State::US915(state) => state.get_join_frequency(random),
            State::CN470(state) => state.get_join_frequency(random),
            State::EU868(state) => state.get_join_frequency(random),

        }
    }
    fn get_data_frequency(&mut self, random: u8) -> u32 {
        match &mut self.state {
            State::US915(state) => state.get_data_frequency(random),
            State::CN470(state) => state.get_data_frequency(random),
            State::EU868(state) => state.get_data_frequency(random),

        }
    }
    fn get_join_accept_frequency1(&self) -> u32 {
        match &self.state {
            State::US915(state) => state.get_join_accept_frequency1(),
            State::CN470(state) => state.get_join_accept_frequency1(),
            State::EU868(state) => state.get_join_accept_frequency1(),

        }
    }
    fn get_rxwindow1_frequency(&self) -> u32 {
        match &self.state {
            State::US915(state) => state.get_rxwindow1_frequency(),
            State::CN470(state) => state.get_rxwindow1_frequency(),
            State::EU868(state) => state.get_rxwindow1_frequency(),

        }
    }
    fn get_join_accept_delay1(&self) -> u32 {
        match &self.state {
            State::US915(state) => state.get_join_accept_delay1(),
            State::CN470(state) => state.get_join_accept_delay1(),
            State::EU868(state) => state.get_join_accept_delay1(),
        }
    }
    fn get_join_accept_delay2(&self) -> u32 {
        match &self.state {
            State::US915(state) => state.get_join_accept_delay2(),
            State::CN470(state) => state.get_join_accept_delay2(),
            State::EU868(state) => state.get_join_accept_delay2(),

        }
    }
    fn get_receive_delay1(&self) -> u32 {
        match &self.state {
            State::US915(state) => state.get_receive_delay1(),
            State::CN470(state) => state.get_receive_delay1(),
            State::EU868(state) => state.get_receive_delay1(),

        }
    }
    fn get_receive_delay2(&self) -> u32 {
        match &self.state {
            State::US915(state) => state.get_receive_delay2(),
            State::CN470(state) => state.get_receive_delay2(),
            State::EU868(state) => state.get_receive_delay2(),
        }
    }
}

pub trait RegionHandler {
    fn process_join_accept<T: core::convert::AsRef<[u8]>,C>(&mut self, join_accept: &DecryptedJoinAcceptPayload<T, C>);
    fn set_channel_mask(&mut self, channel_mask: ChannelMask);
    fn set_subband(&mut self, subband: u8);
    fn get_join_frequency(&mut self, random: u8) -> u32;
    fn get_data_frequency(&mut self, random: u8) -> u32;
    fn get_join_accept_frequency1(&self) -> u32;
    fn get_rxwindow1_frequency(&self) -> u32;
    fn get_join_accept_delay1(&self) -> u32;
    fn get_join_accept_delay2(&self) -> u32;
    fn get_receive_delay1(&self) -> u32;
    fn get_receive_delay2(&self) -> u32;
}