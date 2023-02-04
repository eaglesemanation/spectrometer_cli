use crate::{
    command::Command,
    error::{Error, Result},
    flags::{BaudRate, TriggerMode},
    response::{
        parser::{align_response, parse_response},
        Frame, Response, VersionDetails,
    },
    IoAdapter,
};
use core::{mem::size_of, iter, iter::Extend};
use scopeguard::guard;

// Sized as 2 responses in case of really unfortunate initial misalignment
const READ_BUF_SIZE: usize = size_of::<Response>() * 2;

pub struct CCD<IO>
where
    IO: IoAdapter,
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
    IO: IoAdapter,
{
    pub(crate) fn new(io: IO) -> Self {
        CCD {
            io,
            buf: [0; READ_BUF_SIZE],
            top: 0,
            aligned: false,
        }
    }

    fn fill_buffer(&mut self) -> Result<()> {
        self.aligned = false;
        let read_bytes = self.io.read(&mut self.buf[self.top..])?;
        self.top += read_bytes;
        Ok(())
    }

    // Tries to align data in read buffer to a recognized package head
    fn align_buffer(&mut self) {
        if let Ok((tail, _)) = align_response(&self.buf[..self.top]) {
            let consumed = self.top - tail.len();
            self.buf.rotate_left(consumed);
            self.top -= consumed;
            self.aligned = true;
        }
    }

    fn send_package(&mut self, cmd: Command) -> Result<()> {
        self.io.write_all(&cmd.encode())?;
        Ok(())
    }

    fn receive_package(&mut self) -> Result<Response> {
        loop {
            log::trace!("Filling read buffer");
            self.fill_buffer()?;
            log::trace!("Parsing response");
            match parse_response(&self.buf[..self.top]) {
                Ok((tail, resp)) => {
                    log::trace!("Successfuly parsed a package, freeing space in read buffer");
                    let consumed = self.top - tail.len();
                    self.buf.rotate_left(consumed);
                    self.top -= consumed;
                    return Ok(resp);
                }
                Err(nom::Err::Incomplete(needed)) => {
                    log::trace!("Response is incomplete, amount of data needed: {:?}", needed);
                    // TODO: Add a timeout / retry count if package never fully arrives
                    continue;
                }
                // TODO: Pass through parser errors when implemented correctly
                Err(_) => {
                    if !self.aligned {
                        log::trace!("Failed to parse a package, trying to realign");
                        self.align_buffer();
                    } else {
                        return Err(Error::InvalidData);
                    }
                }
            }
        }
    }

    pub fn set_avg_time(&mut self, t: u8) -> Result<()> {
        log::debug!("Sending a SetAverageTime package with t = {}", t);
        self.send_package(Command::SetAverageTime(t))
    }

    pub fn get_avg_time(&mut self) -> Result<u8> {
        log::debug!("Sending a GetAverageTime package");
        self.send_package(Command::GetAverageTime)?;
        log::debug!("Waiting for a response");
        match self.receive_package()? {
            Response::AverageTime(t) => {
                log::debug!("Recieved a AverageTime package with t = {}", t);
                Ok(t)
            },
            r => Err(Error::UnexpectedResponse(r.into())),
        }
    }

    // TODO: Figure out difference between Average, Integration and Exposure time
    pub fn set_exp_time(&mut self, t: u16) -> Result<()> {
        log::debug!("Sending a SetIntegrationTime package with t = {}", t);
        self.send_package(Command::SetIntegrationTime(t))
    }

    pub fn get_exp_time(&mut self) -> Result<u16> {
        log::debug!("Sending a GetExposureTime package");
        self.send_package(Command::GetExposureTime)?;
        log::debug!("Waiting for a response");
        match self.receive_package()? {
            Response::ExposureTime(t) => {
                log::debug!("Recieved a ExposureTime package with t = {}", t);
                Ok(t)
            },
            r => Err(Error::UnexpectedResponse(r.into())),
        }
    }

    pub fn set_trigger_mode(&mut self, mode: TriggerMode) -> Result<()> {
        log::debug!("Sending a SetTrigerMode package with mode = {:?}", mode);
        self.send_package(Command::SetTrigerMode(mode))
    }

    /// Sets baud rate on UART pins (does not affect USB ACM)
    pub fn set_baudrate(&mut self, baud: BaudRate) -> Result<()> {
        log::debug!("Sending a SetSerialBaudRate package");
        self.send_package(Command::SetSerialBaudRate(baud))
    }

    /// Gets current baud rate on UART pins
    pub fn get_baudrate(&mut self) -> Result<BaudRate> {
        log::debug!("Sending a GetSerialBaudRate package");
        self.send_package(Command::GetSerialBaudRate)?;
        log::debug!("Waiting for a response");
        match self.receive_package()? {
            Response::SerialBaudRate(b) => {
                log::debug!("Recieved a SerialBaudRate package");
                Ok(b)
            },
            r => Err(Error::UnexpectedResponse(r.into())),
        }
    }

    /// Gets CCD version details
    pub fn get_version(&mut self) -> Result<VersionDetails> {
        log::debug!("Sending a GetVersion package");
        self.send_package(Command::GetVersion)?;
        log::debug!("Waiting for a response");
        match self.receive_package()? {
            Response::VersionInfo(d) => {
                log::debug!("Recieved a VersionInfo package");
                Ok(d)
            },
            r => Err(Error::UnexpectedResponse(r.into())),
        }
    }

    /// Takes a single frame from CCD
    pub fn get_frame(&mut self) -> Result<Frame> {
        log::debug!("Sending a SingleRead package");
        self.send_package(Command::SingleRead)?;
        log::debug!("Waiting for a response");
        match self.receive_package()? {
            Response::SingleReading(f) => {
                log::debug!("Recieved a SingleReading package");
                Ok(f)
            },
            r => Err(Error::UnexpectedResponse(r.into())),
        }
    }

    /// Takes `count` frames from CCD and pushes them into buffer, or exits early on an error
    pub fn get_frames<B: Extend<Frame>>(&mut self, buf: &mut B, count: usize) -> Result<()> {
        log::debug!("Sending a ContinuousRead package");
        self.send_package(Command::ContinuousRead)?;
        let mut s = guard(self, |s| {
            log::debug!("Sending a PauseRead package");
            // FIXME: Is it really unrecoverable? Maybe at least add retries or something like that
            s.send_package(Command::PauseRead)
                .expect("Failed to stop continious CCD reading, unrecoverable state");
        });
        log::debug!("Capturing {} frames", count);
        for _ in 0..count {
            log::debug!("Waiting for a response");
            let frame = match s.receive_package()? {
                Response::SingleReading(f) => {
                    log::debug!("Recieved a SingleReading package");
                    f
                },
                r => return Err(Error::UnexpectedResponse(r.into())),
            };
            buf.extend(iter::once(frame))
        }
        Ok(())
    }
}
