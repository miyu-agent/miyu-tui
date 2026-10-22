//! 测试里摆做好的图：做图的活不交给后台线程，排在收件口里，[`finish`] 当场做完（正文排版的测试要看图那几行的样子）。

use std::sync::mpsc::{self, Receiver};

use image::{DynamicImage, RgbaImage};
use ratatui_image::picker::{Picker, ProtocolType};
use ratatui_image::sliced::SlicedProtocol;

use super::worker::{Done, Job, Outcome};
use super::{Drawn, Figures};

/// 能画图的一份，做图的活排进交回的收件口。
pub fn queued(keep: usize) -> (Figures, Receiver<Job>) {
    let (sender, jobs) = mpsc::channel();
    (Figures::with_jobs(Some(sender), keep), jobs)
}

/// 排着的活都做完：每张做成 `px` 像素宽、`rows` 行高的空图（一格 10×20 像素）。
pub fn finish(figures: &mut Figures, jobs: &Receiver<Job>, px: u32, rows: u16) {
    for job in jobs.try_iter() {
        let mut picker = Picker::halfblocks();
        picker.set_protocol_type(ProtocolType::Kitty);
        let image = DynamicImage::ImageRgba8(RgbaImage::new(px, u32::from(rows) * 20));
        let protocol = SlicedProtocol::new(&picker, image, None).unwrap();
        figures.done(Done {
            key: job.key,
            result: Outcome::Drawn(Drawn {
                protocol,
                rows,
                zoom: None,
            }),
        });
    }
}
