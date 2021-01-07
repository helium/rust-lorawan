use lorawan_encoding::maccommands::ChannelMask;

mod us915;

pub use us915::US915;

pub trait Configuration {
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