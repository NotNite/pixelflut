use std::error::Error;

use clap::{ArgEnum, Parser};
use image::imageops::FilterType;
use image::{GenericImageView, Pixel};
use rand::prelude::SliceRandom;

use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

// TODO: don't hardcode this lmao
const HOST: &str = "lmaobox.n2.pm:33333";
const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;

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
    threads: u8,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let img = image::open(&args.image_path).expect("Couldn't load image file");

    if args.w.is_some() && args.h.is_some() {
        img.resize(args.w.unwrap(), args.h.unwrap(), FilterType::Triangle);
    }

    let image_width = img.width();
    let image_height = img.height();

    let mut x: u32 = args.x;
    let mut y: u32 = args.y;

    if let Some(pos) = args.position {
        (x, y) = match pos {
            ImagePosition::TopLeft => (0, 0),
            ImagePosition::TopMiddle => ((WIDTH - image_width) / 2, 0),
            ImagePosition::TopRight => (WIDTH - image_width, 0),

            ImagePosition::MiddleLeft => (0, (HEIGHT - image_height) / 2),
            ImagePosition::Middle => ((WIDTH - image_width) / 2, (HEIGHT - image_height) / 2),
            ImagePosition::MiddleRight => (WIDTH - image_width, (HEIGHT - image_height) / 2),

            ImagePosition::BottomLeft => (0, HEIGHT - image_height),
            ImagePosition::BottomMiddle => ((WIDTH - image_width) / 2, HEIGHT - image_height),
            ImagePosition::BottomRight => (WIDTH - image_width, HEIGHT - image_height),
        };
    }

    let mut handles = vec![];

    for _ in 0..args.threads {
        // shamelessly stolen from anna
        let mut pixels: Vec<_> = img
            .pixels()
            .filter(|(_, _, col)| col.channels()[3] == 255)
            .collect();
        pixels.shuffle(&mut rand::thread_rng());

        let mut stream = TcpStream::connect(HOST).await.expect("Could not connect");

        let handle = tokio::task::spawn(async move {
            loop {
                for (px, py, color) in &pixels {
                    let channels = color.channels();

                    let hex = format!("{:02x}{:02x}{:02x}", channels[0], channels[1], channels[2]);
                    let command = format!("PX {} {} {}\n", x + px, y + py, hex);

                    stream
                        .write_all(command.as_bytes())
                        .await
                        .expect("Failed to write to stream");
                }
            }
        });

        handles.push(handle);
    }

    println!("Running, C-c to stop...");
    futures::future::join_all(handles).await;

    Ok(())
}
