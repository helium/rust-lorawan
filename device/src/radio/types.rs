pub enum Bandwidth {
    _125KHZ,
    _250KHZ,
    _500KHZ,
}

pub enum SpreadingFactor {
    _7,
    _8,
    _9,
    _10,
    _11,
    _12,
}

pub enum CodingRate {
    _4_5,
    _4_6,
    _4_7,
    _4_8,
}

pub struct RfConfig {
    pub frequency: u32,
    pub bandwidth: Bandwidth,
    pub spreading_factor: SpreadingFactor,
    pub coding_rate: CodingRate,
}

pub struct TxConfig {
    pub pw: i8,
    pub rf: RfConfig,
}

#[derive(Copy, Clone, Debug)]
pub struct RxQuality {
    rssi: i16,
    snr: i8,
}
