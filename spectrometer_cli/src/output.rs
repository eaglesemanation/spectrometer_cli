use ccd_lcamv06::Frame;
use time::{OffsetDateTime, macros::format_description, format_description::FormatItem};
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

struct ChartData<'a> {
    frame: &'a Frame,
    idx: usize,
    timestamp: OffsetDateTime,
}

const TIMESTAMP_FORMAT: &[FormatItem<'static>] = format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");

fn draw_frame<'a, DB: DrawingBackend>(
    root: &DrawingArea<DB, plotters::coord::Shift>,
    data: ChartData<'a>,
) -> Result<()>
where
    DB::ErrorType: 'static,
{
    root.fill(&WHITE)?;

    log::trace!("Drawing chart axes");
    let mut chart = ChartBuilder::on(root)
        .caption(
            format!(
                "Frame #{} taken at {}",
                data.idx,
                data.timestamp.format(TIMESTAMP_FORMAT)?
            ),
            ("sans-serif", (5).percent()),
        )
        .set_label_area_size(LabelAreaPosition::Left, (8).percent())
        .set_label_area_size(LabelAreaPosition::Bottom, (5).percent())
        .build_cartesian_2d(0..data.frame.len(), 0u32..100_000u32)?;

    log::trace!("Writing chart axes labels");
    chart
        .configure_mesh()
        .x_desc("Pixel #")
        .y_desc("Inverse intensity")
        .draw()?;

    log::trace!("Drawing frame as a line chart");
    chart.draw_series(LineSeries::new(
        data.frame.iter().enumerate().map(|(x, y)| (x, *y as u32)),
        BLACK,
    ))?;

    log::trace!("Pushing frame chart to rendering backend");
    root.present()?;

    Ok(())
}

impl Output {
    pub fn write_frame(&self, frame: &Frame) -> Result<()> {
        log::debug!("Saving frame to {:?}", self.output);
        match self.format {
            OutputFormat::Chart => {
                let root =
                    BitMapBackend::new(self.output.as_path(), (1280, 720)).into_drawing_area();
                draw_frame(
                    &root,
                    ChartData {
                        frame,
                        idx: 1,
                        timestamp: OffsetDateTime::now_local()?,
                    },
                )?;
            }
            OutputFormat::Csv => {
                let mut out = File::create(self.output.as_path())?;
                let data = frame_to_csv(frame);
                out.write_all(data.as_bytes())?;
            }
        };
        Ok(())
    }

    pub fn write_frames(&self, frames: &[Frame]) -> Result<()> {
        log::debug!("Saving frames to {:?}", self.output);
        match self.format {
            OutputFormat::Chart => {
                let root = BitMapBackend::gif(self.output.as_path(), (1280, 720), 500)?
                    .into_drawing_area();
                let timestamp = OffsetDateTime::now_local()?;
                for (frame_idx, frame) in frames.iter().enumerate() {
                    draw_frame(
                        &root,
                        ChartData {
                            frame,
                            idx: frame_idx + 1,
                            timestamp,
                        },
                    )?;
                }
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
