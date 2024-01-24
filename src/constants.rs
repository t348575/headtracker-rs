pub const MPU6050_ADDR: u8 = 0x68;
pub const MPU6050_WHOAMI_ADDR: u8 = 0x75;

pub const MASTER_MODE_ENABLE: u8 = 0x6A;
pub const MASTER_MODE_ENABLE_BIT: u8 = 5;

pub const I2C_BYPASS_ENABLE: u8 = 0x37;
pub const I2C_BYPASS_ENABLE_BIT: u8 = 1;

pub const POWER_MGMT_1: u8 = 0x6B;

pub const SLEEP_ENABLED_BIT: u8 = 6;

pub const CLOCK_SELECT_BIT: u8 = 2;
pub const CLOCK_SELECT_LENGTH: u8 = 3;
pub const CLOCK_SOURCE: u8 = 1;

pub const GYRO_CONFIG: u8 = 0x1B;
pub const GYRO_CONFIG_SELECT_BIT: u8 = 4;
pub const GYRO_CONFIG_SELECT_LENGTH: u8 = 2;
pub const GYRO_CONFIG_250: u8 = 0;

pub const ACCEL_CONFIG: u8 = 0x1C;
pub const ACCEL_CONFIG_SELECT_BIT: u8 = 4;
pub const ACCEL_CONFIG_SELECT_LENGTH: u8 = 2;
pub const ACCEL_CONFIG_2G: u8 = 0;

pub const ACCEL_GYRO_READ: u8 = 0x3B;
pub const MAG_READ: u8 = 0x03;

pub const HMC5883L_ADDR: u8 = 0x1E;
pub const HMC5883L_WHOAMI_ADDR: u8 = 0x0A;
pub const HMC5883L_CONFIG_A: u8 = 0;
pub const HMC5883L_CONFIG_B: u8 = 1;

pub const HMC5883L_AVERAGING_8: u8 = 0x03;
pub const HMC5883L_CRA_AVERAGE_BIT: u8 = 6;
pub const HMC5883L_CRA_AVERAGE_LENGTH: u8 = 2;
pub const HMC5883L_RATE_15: u8 = 0x04;
pub const HMC5883L_CRA_RATE_BIT: u8 = 4;
pub const HMC5883L_CRA_RATE_LENGTH: u8 = 3;
pub const HMC5883L_BIAS_NORMAL: u8 = 0x00;
pub const HMC5883L_CRA_BIAS_BIT: u8 = 1;
pub const HMC5883L_CRA_BIAS_LENGTH: u8 = 2;

pub const HMC5883L_GAIN: u8 = 1;
pub const HMC5883L_GAIN_BIT: u8 = 7;
pub const HMC5883L_GAIN_LENGTH: u8 = 3;

pub const HMC5883L_MODE_SINGLE: u8 = 1;
pub const HMC5883L_MODE_REG: u8 = 0x02;
pub const HMC5883L_MODE_REG_BIT: u8 = 1;
pub const HMC5883L_MODE_REG_LENGTH: u8 = 2;
