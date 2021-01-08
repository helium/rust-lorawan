#![allow(dead_code)]
use super::RegionHandler;
use lorawan_encoding::maccommands::ChannelMask;

const UPLINK_CHANNEL_MAP: [[u32; 8]; 8] = [
    [
        902_300_000,
        902_500_000,
        902_700_000,
        902_900_000,
        903_100_000,
        903_300_000,
        903_500_000,
        903_700_000,
    ],
    [
        903_900_000,
        904_100_000,
        904_300_000,
        904_500_000,
        904_700_000,
        904_900_000,
        905_100_000,
        905_300_000,
    ],
    [
        905_500_000,
        905_700_000,
        905_900_000,
        906_100_000,
        906_300_000,
        906_500_000,
        906_700_000,
        906_900_000,
    ],
    [
        907_100_000,
        907_300_000,
        907_500_000,
        907_700_000,
        907_900_000,
        908_100_000,
        908_300_000,
        908_500_000,
    ],
    [
        908_700_000,
        908_900_000,
        909_100_000,
        909_300_000,
        909_500_000,
        909_700_000,
        909_900_000,
        910_100_000,
    ],
    [
        910_300_000,
        910_500_000,
        910_700_000,
        910_900_000,
        911_100_000,
        911_300_000,
        911_500_000,
        911_700_000,
    ],
    [
        911_900_000,
        912_100_000,
        912_300_000,
        912_500_000,
        912_700_000,
        912_900_000,
        913_100_000,
        913_300_000,
    ],
    [
        913_500_000,
        913_700_000,
        913_900_000,
        914_100_000,
        914_300_000,
        914_500_000,
        914_700_000,
        914_900_000,
    ],
];

const DOWNLINK_CHANNEL_MAP: [u32; 8] = [
    922_300_000,
    923_900_000,
    924_500_000,
    925_100_000,
    925_700_000,
    926_300_000,
    926_900_000,
    927_500_000,
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
pub struct US915 {
    subband: Option<u8>,
    last_tx: (u8, u8),
}

impl US915 {
    pub fn new() -> US915 {
        Self::default()
    }
}
impl RegionHandler for US915 {
    fn process_join_accept<T: core::convert::AsRef<[u8]>,C>(&mut self, _join_accept: &super::DecryptedJoinAcceptPayload<T, C>) {
        // placeholder
    }
    fn set_channel_mask(&mut self, _chmask: ChannelMask) {
        // one day this should truly be handled
    }

    fn set_subband(&mut self, subband: u8) {
        self.subband = Some(subband);
    }

    fn get_join_frequency(&mut self, random: u8) -> u32 {
        let subband_channel = random & 0b111;
        let subband = if let Some(subband) = &self.subband {
            subband - 1
        } else {
            (random >> 3) & 0b111
        };
        self.last_tx = (subband, subband_channel);
        UPLINK_CHANNEL_MAP[subband as usize][subband_channel as usize]
    }

    fn get_data_frequency(&mut self, random: u8) -> u32 {
        let subband_channel = random & 0b111;
        let subband = if let Some(subband) = &self.subband {
            subband - 1
        } else {
            (random >> 3) & 0b111
        };
        self.last_tx = (subband, subband_channel);
        UPLINK_CHANNEL_MAP[subband as usize][subband_channel as usize]
    }

    fn get_join_accept_frequency1(&self) -> u32 {
        DOWNLINK_CHANNEL_MAP[self.last_tx.1 as usize]
    }

    fn get_rxwindow1_frequency(&self) -> u32 {
        DOWNLINK_CHANNEL_MAP[self.last_tx.1 as usize]
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
