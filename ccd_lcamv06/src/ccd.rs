use crate::{
    command::Command,
    error::{Error, Result},
    flags::{BaudRate, TriggerMode},
    response::{
        parser::{align_response, parse_response},
        Frame, Response, VersionDetails,
    },
};
use core::mem::size_of;
use scopeguard::guard;
use std::io::{Read, Write};

// Sized as 2 responses in case of really unfortunate initial misalignment
const READ_BUF_SIZE: usize = size_of::<Response>() * 2;

pub struct CCD<IO>
where
    IO: Read + Write,
{
    io: IO,
    // Read buffer
    buf: [u8; READ_BUF_SIZE],
    // Points to the top of buffer
    top: usize,
    // Keeps track if buffer was aligned after latest buffer read
    aligned: bool,
}

impl<IO> CCD<IO>
where
    IO: Read + Write,
{
    pub fn new(io: IO) -> Self {
        CCD {
            io,
            buf: [0; READ_BUF_SIZE],
            top: 0,
            aligned: false,
        }
    }

    fn send_package(&mut self, cmd: Command) -> Result<()> {
        self.io.write_all(&cmd.encode())?;
        Ok(())
    }

    fn fill_buffer(&mut self) -> Result<()> {
        self.aligned = false;
        let read_bytes = self.io.read(&mut self.buf[self.top..])?;
        self.top += read_bytes;
        Ok(())
    }

    // Tries to align data in read buffer to a recognized package head
    fn align_buffer(&mut self) {
        if let Ok((tail, _)) = align_response(&mut self.buf[..=self.top]) {
            let consumed = self.top - tail.len() + 1;
            self.buf.rotate_left(consumed);
            self.top -= consumed;
            self.aligned = true;
        }
    }

    fn receive_package(&mut self) -> Result<Response> {
        loop {
            self.fill_buffer()?;
            match parse_response(&self.buf[..=self.top]) {
                Ok((tail, resp)) => {
                    let consumed = self.top - tail.len() + 1;
                    self.buf.rotate_left(consumed);
                    self.top -= consumed;
                    return Ok(resp);
                }
                Err(nom::Err::Incomplete(_)) => {
                    // TODO: Add a timeout / retry count if package never fully arrives
                    continue;
                }
                // TODO: Pass through parser errors when implemented correctly
                Err(_) => {
                    if !self.aligned {
                        self.align_buffer();
                    } else {
                        return Err(Error::InvalidData);
                    }
                }
            }
        }
    }

    pub fn set_avg_time(&mut self, t: u8) -> Result<()> {
        self.send_package(Command::SetAverageTime(t))
    }

    pub fn get_avg_time(&mut self) -> Result<u8> {
        self.send_package(Command::GetAverageTime)?;
        match self.receive_package()? {
            Response::AverageTime(t) => Ok(t),
            _ => Err(Error::UnexpectedResponse),
        }
    }

    // TODO: Figure out difference between Average, Integration and Exposure time
    pub fn set_exp_time(&mut self, t: u16) -> Result<()> {
        self.send_package(Command::SetIntegrationTime(t))
    }

    pub fn get_exp_time(&mut self) -> Result<u16> {
        self.send_package(Command::GetExposureTime)?;
        match self.receive_package()? {
            Response::ExposureTime(t) => Ok(t),
            _ => Err(Error::UnexpectedResponse),
        }
    }

    pub fn set_trigger_mode(&mut self, mode: TriggerMode) -> Result<()> {
        self.send_package(Command::SetTrigerMode(mode))
    }

    /// Sets baud rate on UART pins (does not affect USB ACM)
    pub fn set_baudrate(&mut self, baud: BaudRate) -> Result<()> {
        self.send_package(Command::SetSerialBaudRate(baud))
    }

    /// Gets current baud rate on UART pins
    pub fn get_baudrate(&mut self) -> Result<BaudRate> {
        self.send_package(Command::GetSerialBaudRate)?;
        match self.receive_package()? {
            Response::SerialBaudRate(b) => Ok(b),
            _ => Err(Error::UnexpectedResponse),
        }
    }

    /// Gets CCD version details
    pub fn get_version(&mut self) -> Result<VersionDetails> {
        self.send_package(Command::GetVersion)?;
        match self.receive_package()? {
            Response::VersionInfo(d) => Ok(d),
            _ => Err(Error::UnexpectedResponse),
        }
    }

    /// Takes a single frame from CCD
    pub fn get_frame(&mut self) -> Result<Frame> {
        self.send_package(Command::SingleRead)?;
        match self.receive_package()? {
            Response::SingleReading(f) => Ok(f),
            _ => Err(Error::UnexpectedResponse),
        }
    }

    /// Takes frames from CCD until buffer is filled or got an error while receiving package
    pub fn get_frames(&mut self, buf: &mut [Frame]) -> Result<()> {
        self.send_package(Command::ContinuousRead)?;
        let mut s = guard(self, |s| {
            // FIXME: Is it really unrecoverable? Maybe at least add retries or something like that
            s.send_package(Command::PauseRead)
                .expect("Failed to stop continious CCD reading, unrecoverable state");
        });
        for i in 0..buf.len() {
            buf[i] = match s.receive_package()? {
                Response::SingleReading(f) => f,
                _ => return Err(Error::UnexpectedResponse),
            }
        }
        Ok(())
    }
}
