use std::thread::sleep;
use std::time::Duration;
use std::{str, thread};

use rppal::gpio::{Gpio, InputPin, OutputPin};
use rppal::uart::{Parity, Uart};

const M0_PIN: u8 = 22;
const M1_PIN: u8 = 27;
const AUX_PIN: u8 = 7;

// Define UART Baud Rates as enum
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum UartBaudRate {
    Baud1200 = 0x00,
    Baud2400 = 0x20,
    Baud4800 = 0x40,
    Baud9600 = 0x60,
    Baud19200 = 0x80,
    Baud38400 = 0xA0,
    Baud57600 = 0xC0,
    Baud115200 = 0xE0,
}

// Define Package Sizes as enum
#[derive(Debug, Clone, Copy)]
pub enum PackageSize {
    Size240Byte = 0x00,
    Size128Byte = 0x40,
    Size64Byte = 0x80,
    Size32Byte = 0xC0,
}

// Define Power Levels as enum
#[derive(Debug, Clone, Copy)]
pub enum PowerLevel {
    Power22dBm = 0x00,
    Power17dBm = 0x01,
    Power13dBm = 0x02,
    Power10dBm = 0x03,
}
#[derive(Debug, Clone, Copy)]
pub enum AirSpeed {
    Speed1200 = 0x01,
    Speed2400 = 0x02,
    Speed4800 = 0x03,
    Speed9600 = 0x04,
    Speed19200 = 0x05,
    Speed38400 = 0x06,
    Speed62500 = 0x07,
}

#[allow(dead_code)]
pub struct Sx1262UartE22 {
    m0: OutputPin,
    m1: OutputPin,
    aux: InputPin,
    uart: Uart,
    cfg: Vec<u8>,
    addr: u16,        // own address
    start_freq: u16,  // Start frequency of LoRa module
    offset_freq: u16, // Offset between start and end frequency of LoRa module
}

pub struct ReceiveResult {
    pub data: Vec<u8>,
    pub rssi: Option<i16>,
    pub snr: Option<u8>,
}

impl Sx1262UartE22 {
    pub fn new(serial_port: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let gpio = Gpio::new()?;
        let m0 = gpio.get(M0_PIN)?.into_output();
        let m1 = gpio.get(M1_PIN)?.into_output();
        let aux = gpio.get(AUX_PIN)?.into_input();
        let cfg = vec![0xC2, 0x00, 0x09, 0x00, 0x00, 0x00, 0x62, 0x00, 0x12, 0x43, 0x00, 0x00];

        let uart = Uart::with_path(serial_port, 9600, Parity::None, 8, 1)?;
        // Set read mode to blocking with a timeout of 100ms
        // If a length of 255 is reached, the read function will return immediately
        // uart.set_read_mode(255, Duration::from_millis(100))?;

        Ok(Sx1262UartE22 {
            m0,
            m1,
            aux,
            uart,
            cfg,
            addr: 65535,     // Initialize addr
            start_freq: 850, // Initialize start_freq for E22-900T22S by default or adjust based on module
            offset_freq: 18, // Initialize offset_freq
        })
    }

    pub fn set_mode(&mut self, mode: (bool, bool)) {
        match mode {
            (false, false) => {
                self.m0.set_low();
                self.m1.set_low();
            }
            (true, false) => {
                self.m0.set_high();
                self.m1.set_low();
            }
            (false, true) => {
                self.m0.set_low();
                self.m1.set_high();
            }
            (true, true) => {
                self.m0.set_high();
                self.m1.set_high();
            }
        }
    }
    fn write(&mut self, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        self.uart.write(data)?;
        Ok(())
    }

    fn read(&mut self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        sleep(Duration::from_millis(300));
        let mut buf = vec![0; 255];
        let len = self.uart.read(&mut buf)?;
        buf.truncate(len);
        Ok(buf)
    }

    pub fn send(
        &mut self,
        node_addr: u16,
        freq: u32,
        message_payload: &Vec<u8>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let offset_frequency = if freq > 850 { freq - 850 } else { freq - 410 };

        let high_addr = (node_addr >> 8) as u8;
        let low_addr = (node_addr & 0xFF) as u8;
        let own_high_addr = (self.addr >> 8) as u8;
        let own_low_addr = (self.addr & 0xFF) as u8;
        // println!("{:?}", message_payload);
        let data = [
            &[
                high_addr,
                low_addr,
                offset_frequency as u8,
                own_high_addr,
                own_low_addr,
                self.offset_freq as u8,
            ],
            message_payload.as_slice(),
        ]
        .concat();

        self.set_mode((false, false));
        thread::sleep(Duration::from_millis(100));
        self.write(&data)?;
        thread::sleep(Duration::from_millis(100));
        Ok(())
    }

    pub fn receive(&mut self) -> Option<ReceiveResult> {
        if let Ok(r_buff) = self.read() {
            if !r_buff.is_empty() {
                let _addr = ((r_buff[0] as u16) << 8) + r_buff[1] as u16;
                let _freq = r_buff[2] as u16 + self.start_freq;
                let noise = r_buff[r_buff.len() - 1];
                let rssi = self.get_channel_rssi().unwrap();
                return Some(ReceiveResult {
                    data: r_buff[3..r_buff.len()].to_vec(),
                    rssi: Some(rssi),
                    snr: Some(noise),
                });
            } else {
                return None;
            }
        }
        None
    }

    #[allow(dead_code)]
    pub fn set(
        &mut self,
        freq: u32,
        addr: u16,
        net_id: u16,
        power: PowerLevel,
        rssi: bool,
        air_speed: AirSpeed,
        buffer_size: PackageSize,
        crypt: u16,
        // relay: bool,
        // lbt: bool,
        // wor: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Set M0 and M1 for configuration
        self.set_mode((false, true));
        thread::sleep(Duration::from_millis(100));

        let high_addr = (addr >> 8) as u8 & 0xFF;
        let low_addr = addr as u8 & 0xFF;
        let net_id_temp = net_id as u8 & 0xFF;
        let offset_freq = if freq > 850 {
            (freq - 850) as u8
        } else {
            (freq - 410) as u8
        };
        self.start_freq = if freq > 850 { 850 } else { 410 };
        self.offset_freq = offset_freq as u16;
        let buffer_size_temp = buffer_size as u8;
        let power_temp = power as u8;
        let rssi_temp = if rssi { 0x80 } else { 0x00 };
        let enable_noise = (0b1 << 5) as u8;
        // Encryption keys split into high and low bytes
        let l_crypt = (crypt & 0xFF) as u8;
        let h_crypt = ((crypt >> 8) & 0xFF) as u8;

        // Prepare configuration command
        let cfg_cmd: Vec<u8> = vec![
            0xC2,
            0x00,
            0x09,
            high_addr,
            low_addr,
            net_id_temp,
            UartBaudRate::Baud9600 as u8 | air_speed as u8, // Air speed mapping
            // will enable to read noise rssi value when add 0x20 as follow
            buffer_size_temp | power_temp | enable_noise, // Combined buffer size, power
            offset_freq,                                  // Frequency adjustment
            0x43 | rssi_temp,                             // Additional RSSI configuration
            h_crypt,
            l_crypt, // Encryption key bytes
        ];

        // Write cfg_cmd to UART
        let mut attempt = 0;

        while attempt < 2 {
            self.write(&cfg_cmd)?; // Assuming cfg_cmd is the configuration command
            thread::sleep(Duration::from_millis(200)); // Wait for the device to process the command

            let r_buff = self.read()?;

            if r_buff[0] == 0xC1 {
                // Acknowledgment received
                // If needed, process r_buff further
                break; // Exit the loop on successful acknowledgment
            } else {
                eprintln!("Setting failed, trying again...");
                thread::sleep(Duration::from_millis(200));
                if attempt == 1 {
                    eprintln!("Setting failed, press Esc to exit and try again");
                    // Additional error handling or retries can be implemented here
                }
            }

            attempt += 1;
        }
        thread::sleep(Duration::from_millis(200));
        // Reset M0 and M1 to normal operation mode
        self.set_mode((false, false));

        thread::sleep(Duration::from_millis(100));

        Ok(())
    }

    // Function to get the channel RSSI
    pub fn get_channel_rssi(&mut self) -> Result<i16, Box<dyn std::error::Error>> {
        // Set module to normal operation mode
        self.set_mode((false, false));
        // Wait a little bit for the module to be ready
        sleep(Duration::from_millis(10));

        // Send command to read RSSI
        self.write(&[0xC0, 0xC1, 0xC2, 0xC3, 0x00, 0x02])?;

        // Read the response
        sleep(Duration::from_millis(10));
        let response = self.read()?;

        // Check if the response is valid
        if response.len() >= 5 && response[0] == 0xC1 && response[1] == 0x00 && response[2] == 0x02 {
            let rssi_value = 256 - response[3] as i16;
            println!("the current noise rssi value: -{}dBm", rssi_value);
            return Ok(rssi_value);
        }

        println!("receive rssi value fail");
        Err("Failed to get channel RSSI".into())
    }
}
