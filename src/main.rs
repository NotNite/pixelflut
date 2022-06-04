use clap::{ArgEnum, Parser};
use image::{imageops::FilterType, GenericImageView, Pixel};
use std::{
    io::{self, Write},
    net::TcpStream,
};

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
    threads: u32,
}

pub struct Pixelflut {
    stream: TcpStream,
}

impl Pixelflut {
    pub fn new(host: &str) -> io::Result<Pixelflut> {
        let stream = TcpStream::connect(host)?;
        Ok(Self { stream })
    }

    pub fn write(&mut self, x: u32, y: u32, color: (u8, u8, u8)) -> io::Result<()> {
        let hex = format!("{:02x}{:02x}{:02x}", color.0, color.1, color.2);
        let command = format!("PX {} {} {}\n", x, y, hex);
        self.stream.write_all(command.as_bytes())
    }
}

fn main() {
    let args = Args::parse();

    let img = image::open(&args.image_path).expect("Couldn't load image file");

    if let (Some(w), Some(h)) = (args.w, args.h) {
        img.resize(w, h, FilterType::Triangle);
    }

    let image_width = img.width();
    let image_height = img.height();

    let (x, y) = match args.position {
        Some(ImagePosition::TopLeft) => (0, 0),
        Some(ImagePosition::TopMiddle) => ((WIDTH - image_width) / 2, 0),
        Some(ImagePosition::TopRight) => (WIDTH - image_width, 0),

        Some(ImagePosition::MiddleLeft) => (0, (HEIGHT - image_height) / 2),
        Some(ImagePosition::Middle) => ((WIDTH - image_width) / 2, (HEIGHT - image_height) / 2),
        Some(ImagePosition::MiddleRight) => (WIDTH - image_width, (HEIGHT - image_height) / 2),

        Some(ImagePosition::BottomLeft) => (0, HEIGHT - image_height),
        Some(ImagePosition::BottomMiddle) => ((WIDTH - image_width) / 2, HEIGHT - image_height),
        Some(ImagePosition::BottomRight) => (WIDTH - image_width, HEIGHT - image_height),

        _ => (args.x, args.y),
    };

    let handles: Vec<_> = (0..args.threads)
        .map(|idx| {
            let height = image_height / args.threads;
            let height_offset = idx * height;
            let new_img = img.crop_imm(0, height_offset, image_width, height);

            std::thread::spawn(move || loop {
                let mut pixelflut = Pixelflut::new(HOST).expect("failed to connect to pixelflut");
                for (px, py, color) in new_img.pixels() {
                    let col = color.channels();

                    pixelflut
                        .write(x + px, y + height_offset + py, (col[0], col[1], col[2]))
                        .expect("failed to write to pixelflut");
                }
            })
        })
        .collect();

    println!("Running, C-c to stop...");
    for handle in handles {
        handle.join().unwrap();
    }
}
