use embassy_stm32::{
    peripherals::{DMA2_CH5, DMA2_CH7, USART1},
    usart::Uart,
};
use embassy_time::{Duration, Timer};
use heapless::String;
use rtt_target::{rprint, rprintln};

use crate::util::{self, StringFromBufError};

type UartType<'a> = Uart<'a, USART1, DMA2_CH7, DMA2_CH5>;

const MSG_START: [u8; 3] = [0xAC, 0xFF, 0xAC];
const MSG_END: [u8; 3] = [0xFF, 0xAC, 0xFF];

pub struct Wifi<'a> {
    serial: UartType<'a>,
    chip_ready: bool,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum WifiError {
    Rx,
    Tx,
    BufWrite,
    CouldNotConnectToChip,
    CouldNotWriteAsPassthrough,
    CouldNotConnectUDP,
    CommandFailed,
}

impl From<StringFromBufError> for WifiError {
    fn from(_: StringFromBufError) -> Self {
        WifiError::BufWrite
    }
}

impl<'a> Wifi<'a> {
    pub fn new(serial: UartType<'a>) -> Self {
        Self {
            serial,
            chip_ready: false,
        }
    }

    pub async fn setup(&mut self) -> Result<bool, WifiError> {
        _ = self.exit_passthrough().await;
        _ = self.set_normal_mode().await;
        self.chip_ready = self.test().await?;
        if !self.chip_ready {
            return Err(WifiError::CouldNotConnectToChip);
        }

        self.set_station_mode().await?;
        self.check_connected().await
    }

    pub async fn test(&mut self) -> Result<bool, WifiError> {
        let command = "AT\r\n";
        let res = self.send_recv::<2, 8>(command, 2).await?;

        if res != "OK" {
            return Ok(false);
        }
        Ok(true)
    }

    pub async fn set_station_mode(&mut self) -> Result<(), WifiError> {
        let command = "AT+CWMODE=1\r\n";
        let res = self.send_recv::<2, 17>(command, 2).await?;

        if res != "OK" {
            return Err(WifiError::CommandFailed);
        }
        Timer::after(Duration::from_millis(20)).await;
        Ok(())
    }

    pub async fn check_connected(&mut self) -> Result<bool, WifiError> {
        let command = "AT+CWSTATE?\r\n";
        let res = self.send_recv::<17, 30>(command, 0).await?;

        if res != "+CWSTATE:2,\"Jose\"" {
            return Ok(false);
        }
        Timer::after(Duration::from_millis(20)).await;
        Ok(true)
    }

    pub async fn connect(&mut self) -> Result<bool, WifiError> {
        let command = "AT+CWJAP=\"Jose\",\"reetha11\"\r\n";
        let res = self.send_recv::<50, 78>(command, 0).await?;

        let mut got_connected = false;
        let mut got_ip = false;
        let mut got_ok = false;
        for item in res.split("\r\n") {
            if item == "WIFI CONNECTED" {
                got_connected = true;
            } else if item == "WIFI GOT IP" {
                got_ip = true;
            } else if item == "OK" {
                got_ok = true;
            }
        }

        Timer::after(Duration::from_millis(20)).await;
        Ok(got_connected && got_ip && got_ok)
    }

    pub async fn set_passthrough_mode(&mut self) -> Result<(), WifiError> {
        let command = "AT+CIPMODE=1\r\n";
        let res = self.send_recv::<2, 18>(command, 2).await?;
        if res != "OK" {
            return Err(WifiError::CommandFailed);
        }
        Timer::after(Duration::from_millis(20)).await;
        Ok(())
    }

    pub async fn set_normal_mode(&mut self) -> Result<(), WifiError> {
        let command = "AT+CIPMODE=0\r\n";
        let res = self.send_recv::<2, 18>(command, 2).await?;
        if res != "OK" {
            return Err(WifiError::CommandFailed);
        }
        Timer::after(Duration::from_millis(20)).await;
        Ok(())
    }

    pub async fn disconnect_udp(&mut self) -> Result<(), WifiError> {
        let command = "AT+CIPCLOSE\r\n";
        let recv = self.send_recv_with_error::<5, 18>(command).await?;
        if recv != "ERR" && recv != "CLOSE" {
            return Err(WifiError::CommandFailed);
        }
        Timer::after(Duration::from_millis(20)).await;
        Ok(())
    }

    pub async fn start_udp(&mut self) -> Result<(), WifiError> {
        self.disconnect_udp().await?;
        self.set_passthrough_mode().await?;

        let command = "AT+CIPSTART=\"UDP\",\"192.168.0.10\",4242\r\n";
        let res = self.send_recv_with_error::<7, 46>(command).await?;
        if res != "CONNECT" {
            return Err(WifiError::CouldNotConnectUDP);
        }

        let mut buf = [0u8; 6];
        if let Err(_) = self.serial.read(&mut buf).await {
            return Err(WifiError::Rx);
        }

        Timer::after(Duration::from_millis(20)).await;
        self.enter_passthrough().await?;

        Ok(())
    }

    pub async fn enter_passthrough(&mut self) -> Result<(), WifiError> {
        let command = "AT+CIPSEND\r\n";
        let res = self.send_recv::<1, 21>(command, 8).await?;
        if res != ">" {
            return Err(WifiError::CouldNotWriteAsPassthrough);
        }

        Timer::after(Duration::from_millis(20)).await;
        Ok(())
    }

    pub async fn exit_passthrough(&mut self) -> Result<(), WifiError> {
        Timer::after(Duration::from_millis(20)).await;
        if let Err(_) = self.serial.write("+++".as_bytes()).await {
            return Err(WifiError::Tx);
        }
        Timer::after(Duration::from_millis(20)).await;
        Ok(())
    }

    pub async fn send_pos_data(&mut self, data: &[u8]) -> Result<(), WifiError> {
        if let Err(_) = self.serial.write(&MSG_START).await {
            return Err(WifiError::Tx);
        }
        if let Err(_) = self.serial.write(data).await {
            return Err(WifiError::Tx);
        }
        Ok(())
    }

    async fn send_recv_raw(&mut self, command: &str, rx: &mut [u8]) -> Result<(), WifiError> {
        rprintln!("sent command: {}", command);
        if let Err(_) = self.serial.write(command.as_bytes()).await {
            return Err(WifiError::Tx);
        }

        if let Err(_) = self.serial.read(rx).await {
            return Err(WifiError::Rx);
        }
        Ok(())
    }

    async fn send_recv_with_error<const RETURN_SIZE: usize, const BUF_SIZE: usize>(
        &mut self,
        command: &str,
    ) -> Result<String<RETURN_SIZE>, WifiError> {
        let mut buf = [0u8; BUF_SIZE];
        self.send_recv_raw(command, &mut buf).await?;

        let command_len = command.len();
        if "\r\n"
            .as_bytes()
            .eq_ignore_ascii_case(&buf[command_len..command_len + 2])
        {
            return Ok(util::string_from_buf_with_skip(&buf, command_len + 2)?);
        }

        return Ok(util::string_from_buf_with_skip(&buf, command_len)?);
    }

    async fn send_recv<const RETURN_SIZE: usize, const BUF_SIZE: usize>(
        &mut self,
        command: &str,
        skip_extra: usize,
    ) -> Result<String<RETURN_SIZE>, WifiError> {
        let mut buf = [0u8; BUF_SIZE];
        self.send_recv_raw(command, &mut buf).await?;

        rprintln!("\nrecv buf");
        for i in buf {
            rprint!("{} ", i);
        }
        rprintln!("\ndone");
        Ok(util::string_from_buf_with_skip(
            &buf,
            command.len() + skip_extra,
        )?)
    }
}
