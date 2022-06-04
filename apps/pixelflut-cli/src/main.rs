use clap::{ArgEnum, Parser};
use image::{imageops::FilterType, GenericImageView, Pixel};
use rand::prelude::SliceRandom;
use pixelflut::Pixelflut;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ArgEnum, Debug)]
enum ImagePosition {
    TopLeft,
    TopMiddle,
    TopRight,

    MiddleLeft,
    Middle,
    MiddleRight,

    BottomLeft,
    BottomMiddle,
    BottomRight,
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Host to connect to
    host: String,

    /// Path to the image file
    image_path: String,

    /// X coordinate the image gets drawn at
    #[clap(short, default_value_t = 0)]
    x: u32,

    /// Y coordinate the image gets drawn at
    #[clap(short, default_value_t = 0)]
    y: u32,

    /// Width of the image
    #[clap(short)]
    w: Option<u32>,

    /// Height of the image
    #[clap(short)]
    h: Option<u32>,

    /// Position image appears in
    #[clap(short, long, arg_enum, alias = "pos")]
    position: Option<ImagePosition>,

    /// Number of threads
    #[clap(short, long, default_value_t = 1)]
    threads: u32,

    /// Sleep time on each thread in milliseconds, as not to hammer the CPU
    #[clap(short, long, default_value_t = 100)]
    sleep_time: u32,
}

fn calculate_position(
    pf_width: u32,
    pf_height: u32,
    image_width: u32,
    image_height: u32,
    position: &ImagePosition,
) -> (u32, u32) {
    match position {
        ImagePosition::TopLeft => (0, 0),
        ImagePosition::TopMiddle => ((pf_width - image_width) / 2, 0),
        ImagePosition::TopRight => (pf_width - image_width, 0),

        ImagePosition::MiddleLeft => (0, (pf_height - image_height) / 2),
        ImagePosition::Middle => ((pf_width - image_width) / 2, (pf_height - image_height) / 2),
        ImagePosition::MiddleRight => (pf_width - image_width, (pf_height - image_height) / 2),

        ImagePosition::BottomLeft => (0, pf_height - image_height),
        ImagePosition::BottomMiddle => ((pf_width - image_width) / 2, pf_height - image_height),
        ImagePosition::BottomRight => (pf_width - image_width, pf_height - image_height),
    }
}

fn main() {
    let args = Args::parse();

    let (width, height) = Pixelflut::connect(&args.host)
        .and_then(|mut pf| pf.size())
        .expect("failed to connect to pixelflut to get size");

    let img = image::open(&args.image_path).expect("Couldn't load image file");
    if let (Some(w), Some(h)) = (args.w, args.h) {
        img.resize(w, h, FilterType::Triangle);
    }

    let (x, y) = match args.position {
        Some(position) => calculate_position(width, height, img.width(), img.height(), &position),
        None => (args.x, args.y),
    };

    let handles: Vec<_> = (0..args.threads)
        .map(|idx| {
            let height = img.height() / args.threads;
            let height_offset = idx * height;

            let new_img = img.crop_imm(0, height_offset, img.width(), height);
            let host = args.host.clone();
            std::thread::spawn(move || {
                let mut pixelflut =
                    Pixelflut::connect(&host).expect("failed to connect to pixelflut on thread");
                let mut pixels: Vec<_> = new_img
                    .pixels()
                    .filter(|(_, _, col)| col.channels()[3] == 255)
                    .collect();
                pixels.shuffle(&mut rand::thread_rng());

                loop {
                    for (px, py, color) in &pixels {
                        let col = color.channels();

                        pixelflut
                            .write(x + px, y + height_offset + py, (col[0], col[1], col[2]))
                            .expect("failed to write to pixelflut");
                    }

                    std::thread::sleep(std::time::Duration::from_millis(args.sleep_time as u64));
                }
            })
        })
        .collect();

    println!("Running, C-c to stop...");
    for handle in handles {
        handle.join().unwrap();
    }
}
