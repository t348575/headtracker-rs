use core::f32::consts::PI;

use embassy_stm32::i2c::I2c;
use embassy_stm32::peripherals::I2C1;
use embassy_stm32::peripherals::{DMA1_CH0, DMA1_CH6};
use embassy_time::Instant;
use fusion_rs::nalgebra::Vector3;
use fusion_rs::{Ahrs, Vec3};
use rtt_target::rprintln;

use crate::constants::*;
use crate::util::{convert_accel, convert_gyro};

type I2cType<'a> = I2c<'a, I2C1, DMA1_CH0, DMA1_CH6>;

pub struct Gy87<'a> {
    i2c: I2cType<'a>,
    // imu: Madgwick<f64>,
    imu: Ahrs
}

#[derive(Debug, PartialEq, Copy, Clone)]
struct Coords {
    pub x: f64,
    pub y: f64,
    pub z: f64
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct AccelGyro {
    pub accel: Vec3,
    pub gyro: Vec3,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct MovementData {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f64,
    pub pitch: f64,
    pub roll: f64,
}

impl MovementData {
    pub fn serialize(&self) -> [u8; 48] {
        let mut buf = [0u8; 48];
        buf[0..8].clone_from_slice(&self.x.to_le_bytes());
        buf[8..16].clone_from_slice(&self.y.to_le_bytes());
        buf[16..24].clone_from_slice(&self.z.to_le_bytes());
        buf[24..32].clone_from_slice(&self.yaw.to_le_bytes());
        buf[32..40].clone_from_slice(&self.pitch.to_le_bytes());
        buf[40..48].clone_from_slice(&self.roll.to_le_bytes());
        buf
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum Gy87Error {
    BusError(BusError),
    HmcInit(BusError),
    MpuInit(BusError),
    UpdateGetAccelGyro(BusError),
    UpdateMag(BusError),
    UpdateError,
    UnknownMPUDeviceAddr(u8),
    UnknownHMCDeviceAddr([u8; 3]),
}

impl core::fmt::Display for Gy87Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Gy87Error::BusError(e) => write!(f, "{:?}", e),
            Gy87Error::HmcInit(e) => write!(f, "{:?}", e),
            Gy87Error::MpuInit(e) => write!(f, "{:?}", e),
            Gy87Error::UpdateGetAccelGyro(e) => write!(f, "{:?}", e),
            Gy87Error::UpdateMag(e) => write!(f, "{:?}", e),
            Gy87Error::UpdateError => write!(f, "UpdateError"),
            Gy87Error::UnknownMPUDeviceAddr(e) => write!(f, "{:?}", e),
            Gy87Error::UnknownHMCDeviceAddr(e) => write!(f, "{:?}", e),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum BusError {
    BusWrite,
    BusReadWrite,
}

impl<'a> Gy87<'a> {
    pub fn new(i2c: I2cType<'a>) -> Self {
        Self {
            i2c,
            // imu: Madgwick::new(0.00728, 0.1),
            imu: Ahrs::new()
        }
    }

    pub fn start(&mut self) -> Result<(), Gy87Error> {
        // check who am i
        let who_am_i = self.get_byte(MPU6050_ADDR, MPU6050_WHOAMI_ADDR).map_err(|e| Gy87Error::BusError(e))?;
        if who_am_i != MPU6050_ADDR {
            return Err(Gy87Error::UnknownMPUDeviceAddr(who_am_i));
        }

        self.mpu_init().map_err(|e| Gy87Error::MpuInit(e))?;
        self.hmc_init().map_err(|e| Gy87Error::HmcInit(e))?;

        let mut hmc_who_am_i = [0u8; 3];
        self.get_bytes(HMC5883L_ADDR, HMC5883L_WHOAMI_ADDR, &mut hmc_who_am_i).map_err(|e| Gy87Error::BusError(e))?;
        if hmc_who_am_i[0] != 'H' as u8 || hmc_who_am_i[1] != '4' as u8 || hmc_who_am_i[2] != '3' as u8  {
            return Err(Gy87Error::UnknownHMCDeviceAddr(hmc_who_am_i));
        }

        Ok(())
    }

    pub fn get_accel_gyro(&mut self) -> Result<AccelGyro, BusError> {
        let mut rx_buffer = [0u8; 14];
        self.get_bytes(MPU6050_ADDR, ACCEL_GYRO_READ, &mut rx_buffer)?;
        Ok(
            AccelGyro {
                accel: Vector3::new(
                    convert_accel(i16::from_be_bytes(rx_buffer[0..2].try_into().unwrap())),
                    convert_accel(i16::from_be_bytes(rx_buffer[2..4].try_into().unwrap())),
                    convert_accel(i16::from_be_bytes(rx_buffer[4..6].try_into().unwrap())),
                ),
                gyro: Vector3::new(
                    convert_gyro(i16::from_be_bytes(rx_buffer[8..10].try_into().unwrap())),
                    convert_gyro(i16::from_be_bytes(rx_buffer[10..12].try_into().unwrap())),
                    convert_gyro(i16::from_be_bytes(rx_buffer[12..14].try_into().unwrap())),
                )
            }
        )
    }

    pub fn get_mag(&mut self) -> Result<Vec3, BusError> {
        let mut rx_buffer = [0u8; 6];
        self.get_bytes(HMC5883L_ADDR, MAG_READ, &mut rx_buffer)?;

        self.i2c.blocking_write(HMC5883L_ADDR, &[HMC5883L_MODE_REG, HMC5883L_MODE_SINGLE << (HMC5883L_MODE_REG_BIT - HMC5883L_MODE_REG_LENGTH + 1)]).map_err(|_| BusError::BusWrite)?;

        Ok(
            Vector3::new(
                i16::from_be_bytes(rx_buffer[0..2].try_into().unwrap()) as f32,
                i16::from_be_bytes(rx_buffer[4..6].try_into().unwrap()) as f32,
                i16::from_be_bytes(rx_buffer[2..4].try_into().unwrap()) as f32,
            )
        )
    }

    pub fn update(&mut self, prev: &Instant) -> Result<MovementData, Gy87Error> {
        let accel_gyro = self.get_accel_gyro().map_err(|e| Gy87Error::UpdateGetAccelGyro(e))?;
        let mag = self.get_mag().map_err(|e| Gy87Error::UpdateMag(e))?;
        self.imu.update(accel_gyro.gyro, accel_gyro.accel, mag, prev.elapsed().as_micros() as f32 / 1000000.0);
        let euler = self.imu.get_euler();
        Ok(
            MovementData {
                pitch: euler.pitch as f64,
                roll: euler.roll as f64,
                yaw: euler.yaw as f64,
                x: 0.0,
                y: 0.0,
                z: 0.0,
            }
        )
    }

    fn mpu_init(&mut self) -> Result<(), BusError> {
        // set master mode enable
        self.write_bit(MPU6050_ADDR, MASTER_MODE_ENABLE, MASTER_MODE_ENABLE_BIT, false)?;

        // set i2c bypass
        self.write_bit(MPU6050_ADDR, I2C_BYPASS_ENABLE, I2C_BYPASS_ENABLE_BIT, true)?;

        // enable sleep
        self.write_bit(MPU6050_ADDR, POWER_MGMT_1, SLEEP_ENABLED_BIT, false)?;

        // set clock source
        self.write_bits(MPU6050_ADDR, POWER_MGMT_1, CLOCK_SELECT_BIT, CLOCK_SELECT_LENGTH, CLOCK_SOURCE)?;

        // set full scale gyro
        self.write_bits(MPU6050_ADDR, GYRO_CONFIG, GYRO_CONFIG_SELECT_BIT, GYRO_CONFIG_SELECT_LENGTH, GYRO_CONFIG_250)?;

        // set full scale accelerometer
        self.write_bits(MPU6050_ADDR, ACCEL_CONFIG, ACCEL_CONFIG_SELECT_BIT, ACCEL_CONFIG_SELECT_LENGTH, ACCEL_CONFIG_2G)?;

        // enable sleep
        self.write_bit(MPU6050_ADDR, POWER_MGMT_1, SLEEP_ENABLED_BIT, false)?;

        Ok(())
    }

    fn hmc_init(&mut self) -> Result<(), BusError> {
        // magnometer config
        self.i2c.blocking_write(HMC5883L_ADDR, &[HMC5883L_CONFIG_A,
            (HMC5883L_AVERAGING_8 << (HMC5883L_CRA_AVERAGE_BIT - HMC5883L_CRA_AVERAGE_LENGTH + 1)) |
            (HMC5883L_RATE_15     << (HMC5883L_CRA_RATE_BIT - HMC5883L_CRA_RATE_LENGTH + 1)) |
            (HMC5883L_BIAS_NORMAL << (HMC5883L_CRA_BIAS_BIT - HMC5883L_CRA_BIAS_LENGTH + 1))]).map_err(|_| BusError::BusWrite)?;

        // set gain
        self.i2c.blocking_write(HMC5883L_ADDR, &[HMC5883L_CONFIG_B, HMC5883L_GAIN << (HMC5883L_GAIN_BIT - HMC5883L_GAIN_LENGTH + 1)]).map_err(|_| BusError::BusWrite)?;
        
        // set mode
        self.i2c.blocking_write(HMC5883L_ADDR, &[HMC5883L_MODE_REG, HMC5883L_MODE_SINGLE << (HMC5883L_MODE_REG_BIT - HMC5883L_MODE_REG_LENGTH + 1)]).map_err(|_| BusError::BusWrite)?;

        Ok(())
    }

    fn write_bit(&mut self, device: u8, addr: u8, bit: u8, enable: bool) -> Result<(), BusError> {
        let mut data = self.get_byte(device, addr)?;

        data = if enable {
            data | (1 << bit)
        } else {
            data & !(1 << bit)
        };

        self.i2c.blocking_write(device, &[addr, data]).map_err(|_| BusError::BusWrite)?;
        Ok(())
    }

    fn write_bits(&mut self, device: u8, addr: u8, start_bit: u8, length: u8, mut data: u8) -> Result<(), BusError> {
        let mut new_data = self.get_byte(device, addr)?;
        let mask = ((1 << length) - 1) << (start_bit - length + 1);
        data <<= start_bit - length + 1;
        data &= mask;
        new_data &= !mask;
        new_data |= data;

        self.i2c.blocking_write(device, &[addr, new_data]).map_err(|_| BusError::BusWrite)?;
        Ok(())
    }

    fn get_byte(&mut self, device: u8, addr: u8) -> Result<u8, BusError> {
        let mut rx_buffer: [u8; 1] = [0; 1];
        self.i2c.blocking_write_read(device, &[addr], &mut rx_buffer).map_err(|_| BusError::BusReadWrite)?;
        Ok(rx_buffer[0])
    }

    fn get_bytes(&mut self, device: u8, addr: u8, buffer: &mut [u8]) -> Result<(), BusError> {
        self.i2c.blocking_write_read(device, &[addr], buffer).map_err(|_| BusError::BusReadWrite)?;
        Ok(())
    }
}
