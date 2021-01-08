#![allow(dead_code)]
use super::RegionHandler;
use lorawan_encoding::maccommands::ChannelMask;

const UPLINK_MAP: [u32; 96] = [
    470300000,
    470500000,
    470700000,
    470900000,
    471100000,
    471300000,
    471500000,
    471700000,
    471900000,
    472100000,
    472300000,
    472500000,
    472700000,
    472900000,
    473100000,
    473300000,
    473500000,
    473700000,
    473900000,
    474100000,
    474300000,
    474500000,
    474700000,
    474900000,
    475100000,
    475300000,
    475500000,
    475700000,
    475900000,
    476100000,
    476300000,
    476500000,
    476700000,
    476900000,
    477100000,
    477300000,
    477500000,
    477700000,
    477900000,
    478100000,
    478300000,
    478500000,
    478700000,
    478900000,
    479100000,
    479300000,
    479500000,
    479700000,
    479900000,
    480100000,
    480300000,
    480500000,
    480700000,
    480900000,
    481100000,
    481300000,
    481500000,
    481700000,
    481900000,
    482100000,
    482300000,
    482500000,
    482700000,
    482900000,
    483100000,
    483300000,
    483500000,
    483700000,
    483900000,
    484100000,
    484300000,
    484500000,
    484700000,
    484900000,
    485100000,
    485300000,
    485500000,
    485700000,
    485900000,
    486100000,
    486300000,
    486500000,
    486700000,
    486900000,
    487100000,
    487300000,
    487500000,
    487700000,
    487900000,
    488100000,
    488300000,
    488500000,
    488700000,
    488900000,
    489100000,
    489300000,
];

const DOWNLINK_MAP: [u32; 48] = [
    500300000,
500500000,
500700000,
500900000,
501100000,
501300000,
501500000,
501700000,
501900000,
502100000,
502300000,
502500000,
502700000,
502900000,
503100000,
503300000,
503500000,
503700000,
503900000,
504100000,
504300000,
504500000,
504700000,
504900000,
505100000,
505300000,
505500000,
505700000,
505900000,
506100000,
506300000,
506500000,
506700000,
506900000,
507100000,
507300000,
507500000,
507700000,
507900000,
508100000,
508300000,
508500000,
508700000,
508900000,
509100000,
509300000,
509500000,
509700000,
];

const RECEIVE_DELAY1: u32 = 1000;
const RECEIVE_DELAY2: u32 = RECEIVE_DELAY1 + 1000; // must be RECEIVE_DELAY + 1 s
const JOIN_ACCEPT_DELAY1: u32 = 5000;
const JOIN_ACCEPT_DELAY2: u32 = 6000;
const MAX_FCNT_GAP: usize = 16384;
const ADR_ACK_LIMIT: usize = 64;
const ADR_ACK_DELAY: usize = 32;
const ACK_TIMEOUT: usize = 2; // random delay between 1 and 3 seconds

#[derive(Default)]
pub struct CN470 {
    last_tx: u8
}

impl CN470 {
    pub fn new() -> CN470 {
        Self::default()
    }
}
impl RegionHandler for CN470 {
    fn set_channel_mask(&mut self, _chmask: ChannelMask) {
        // one day this should truly be handled
    }

    // no subband setting for CN470
    fn set_subband(&mut self, _subband: u8) {

    }

    fn get_join_frequency(&mut self, random: u8) -> u32 {
        let channel = random & 0b111;
        self.last_tx = channel;
        UPLINK_MAP[channel as usize]
    }

    fn get_data_frequency(&mut self, random: u8) -> u32 {
        let channel = random & 0b111;
        self.last_tx = channel;
        UPLINK_MAP[channel as usize]
    }

    fn get_join_accept_frequency1(&self) -> u32 {
        DOWNLINK_MAP[self.last_tx as usize /2 ]
    }

    fn get_rxwindow1_frequency(&self) -> u32 {
        DOWNLINK_MAP[self.last_tx as usize/ 2 ]
    }

    fn get_join_accept_delay1(&self) -> u32 {
        JOIN_ACCEPT_DELAY1
    }

    fn get_join_accept_delay2(&self) -> u32 {
        JOIN_ACCEPT_DELAY2
    }

    fn get_receive_delay1(&self) -> u32 {
        RECEIVE_DELAY1
    }

    fn get_receive_delay2(&self) -> u32 {
        RECEIVE_DELAY2
    }
}
