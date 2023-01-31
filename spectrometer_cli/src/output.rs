use ccd_lcamv06::Frame;
use clap::{ArgEnum, Args};
use plotters::prelude::*;
use simple_eyre::{eyre::eyre, Result};
use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Args)]
pub struct Output {
    /// Path to a file where readings should be stored
    #[clap(short, long, value_parser = unique_path_parser, value_hint = clap::ValueHint::FilePath)]
    pub output: PathBuf,

    /// File format for reading output
    #[clap(long, value_enum, default_value_t)]
    pub format: OutputFormat,
}

fn unique_path_parser(p: &str) -> Result<PathBuf> {
    let p = Path::new(p);
    if p.try_exists()? {
        Err(eyre!("Path {p:?} already exists"))
    } else {
        Ok(p.to_path_buf())
    }
}

#[derive(ArgEnum, Clone, Default)]
pub enum OutputFormat {
    #[default]
    Chart,
    Csv,
}

fn frame_to_csv(frame: &Frame) -> String {
    log::trace!("Formatting frame as CSV");
    frame
        .iter()
        .map(|pixel| pixel.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn frames_to_csv(frames: &[Frame]) -> String {
    log::trace!("Formatting frames as CSV");
    frames
        .iter()
        .map(frame_to_csv)
        .collect::<Vec<_>>()
        .join("\n")
}

impl Output {
    fn frame_to_chart(&self, frame: &Frame) -> Result<()> {
        log::trace!("Creating bitmap backend");
        let root = BitMapBackend::new(self.output.as_path(), (1280, 720)).into_drawing_area();
        root.fill(&WHITE)?;

        log::trace!("Drawing chart axes");
        let mut chart = ChartBuilder::on(&root)
            .caption(
                format!(
                    "Reading from {}",
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
                ),
                ("sans-serif", (5).percent()),
            )
            .set_label_area_size(LabelAreaPosition::Left, (8).percent())
            .set_label_area_size(LabelAreaPosition::Bottom, (5).percent())
            .build_cartesian_2d(0..frame.len(), 0u32..100_000u32)?;

        log::trace!("Writing chart axes labels");
        chart
            .configure_mesh()
            .x_desc("Pixel #")
            .y_desc("Inverse intensity")
            .draw()?;

        log::trace!("Drawing frame as a line chart");
        chart.draw_series(LineSeries::new(
            frame.iter().enumerate().map(|(x, y)| (x, *y as u32)),
            BLACK,
        ))?;

        log::trace!("Writing chart into a file");
        root.present()?;

        Ok(())
    }

    pub fn write_frame(&self, frame: &Frame) -> Result<()> {
        log::debug!("Saving frame to {:?}", self.output);
        match self.format {
            OutputFormat::Chart => {
                self.frame_to_chart(frame)?;
            }
            OutputFormat::Csv => {
                let mut out = File::create(self.output.as_path())?;
                let data = frame_to_csv(frame);
                out.write_all(data.as_bytes())?;
            }
        };
        Ok(())
    }

    fn frames_to_chart(&self, frames: &[Frame]) -> Result<()> {
        let root = BitMapBackend::gif(self.output.as_path(), (1280, 720), 100)?.into_drawing_area();

        for (frame_idx, frame) in frames.iter().enumerate() {
            root.fill(&WHITE)?;

            let mut chart = ChartBuilder::on(&root)
                .caption(
                    format!(
                        "Reading #{} from {}",
                        frame_idx + 1,
                        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
                    ),
                    ("sans-serif", 20),
                )
                .set_label_area_size(LabelAreaPosition::Left, (8).percent())
                .set_label_area_size(LabelAreaPosition::Bottom, (4).percent())
                .build_cartesian_2d(0..frame.len(), 0u32..10_000u32)?;

            chart
                .configure_mesh()
                .x_desc("Pixel #")
                .y_desc("Inverse intensity")
                .draw()?;

            chart.draw_series(LineSeries::new(
                frame.iter().enumerate().map(|(idx, val)| {
                    (
                        idx.try_into().expect("pixel id is larger than u32"),
                        (*val).into(),
                    )
                }),
                BLACK.stroke_width(3),
            ))?;

            root.present()?;
        }
        Ok(())
    }

    pub fn write_frames(&self, frames: &[Frame]) -> Result<()> {
        log::debug!("Saving frames to {:?}", self.output);
        match self.format {
            OutputFormat::Chart => {
                self.frames_to_chart(frames)?;
            }
            OutputFormat::Csv => {
                let mut out = File::create(self.output.as_path())?;
                let data = frames_to_csv(frames);
                out.write_all(data.as_bytes())?;
            }
        };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccd_lcamv06::FRAME_PIXEL_COUNT;

    #[test]
    fn convert_frame_to_csv() {
        let frame: Frame = [1000; FRAME_PIXEL_COUNT];
        let csv = frame_to_csv(&frame);
        let csv_fields: Vec<_> = csv.split(",").collect();
        assert_eq!(csv_fields[0], "1000");
    }
}
