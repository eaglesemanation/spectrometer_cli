use crate::error::Error;
use core::{
    fmt,
    fmt::{Debug, Display},
};

use arraystring::SmallString;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct VersionDetails {
    hardware_version: SmallString,
    sensor_type: SmallString,
    firmware_version: SmallString,
    serial_number: SmallString,
}

impl VersionDetails {
    pub(crate) fn try_new(
        hw_ver: &str,
        sensor: &str,
        fw_ver: &str,
        serial: &str,
    ) -> Result<VersionDetails, Error> {
        Ok(VersionDetails {
            hardware_version: SmallString::try_from_str(hw_ver)
                .map_err(|_| Error::VersionDetailTooLong("Hardware version"))?,
            sensor_type: SmallString::try_from_str(sensor)
                .map_err(|_| Error::VersionDetailTooLong("Sensor type"))?,
            firmware_version: SmallString::try_from_str(fw_ver)
                .map_err(|_| Error::VersionDetailTooLong("Firmware version"))?,
            serial_number: SmallString::try_from_str(serial)
                .map_err(|_| Error::VersionDetailTooLong("Serial number"))?,
        })
    }
}

impl<'a> Display for VersionDetails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!(
            concat!(
                "Hardware version: {}\n",
                "Firmware version: {}\n",
                "Sensor type: {}\n",
                "Serial number: {}",
            ),
            &self.hardware_version, &self.firmware_version, &self.sensor_type, &self.serial_number
        ))
    }
}
