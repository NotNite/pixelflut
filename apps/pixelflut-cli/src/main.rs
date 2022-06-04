use clap::{ArgEnum, Parser};
use image::{imageops::FilterType, GenericImageView, Pixel};
use pixelflut::Pixelflut;
use rand::prelude::SliceRandom;

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

    /// Number of tasks
    #[clap(short, long, default_value_t = 1)]
    tasks: u32,
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

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let (width, height) = Pixelflut::connect(&args.host)
        .await
        .expect("failed to connect to pixelflut")
        .size()
        .await
        .expect("failed to get pixelflut size");

    let img = image::open(&args.image_path).expect("Couldn't load image file");
    if let (Some(w), Some(h)) = (args.w, args.h) {
        img.resize(w, h, FilterType::Triangle);
    }

    let (x, y) = match args.position {
        Some(position) => calculate_position(width, height, img.width(), img.height(), &position),
        None => (args.x, args.y),
    };

    let mut pixels: Vec<_> = img
        .pixels()
        .filter(|(_, _, col)| col.channels()[3] == 255)
        .collect();
    pixels.shuffle(&mut rand::thread_rng());

    let handles = pixels
        .chunks(pixels.len() / (args.tasks as usize))
        .map(|pixels| {
            let host = args.host.clone();
            let pixels = pixels.to_vec();

            tokio::spawn(async move {
                let mut pixelflut = Pixelflut::connect(&host)
                    .await
                    .expect("failed to connect to pixelflut on task");

                loop {
                    for (px, py, color) in &pixels {
                        let col = color.channels();

                        pixelflut
                            .write(x + *px, y + *py, (col[0], col[1], col[2]))
                            .await
                            .expect("failed to write to pixelflut");
                    }
                }
            })
        });

    println!("Running, C-c to stop...");
    futures::future::join_all(handles).await;
}
