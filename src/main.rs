use clap::{ArgEnum, Parser};
use image::{imageops::FilterType, GenericImageView, Pixel};
use std::{
    io::{self, BufRead, Write},
    net::TcpStream,
};

// TODO: don't hardcode this lmao
const HOST: &str = "lmaobox.n2.pm:33333";

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
    write: TcpStream,
    read: io::BufReader<TcpStream>,
}

impl Pixelflut {
    pub fn connect(host: &str) -> io::Result<Pixelflut> {
        let stream = TcpStream::connect(host)?;
        let read = io::BufReader::new(stream.try_clone()?);
        Ok(Self {
            write: stream,
            read,
        })
    }

    pub fn size(&mut self) -> io::Result<(u32, u32)> {
        writeln!(self.write, "SIZE")?;

        let mut line = String::new();
        self.read.read_line(&mut line)?;

        let mut iter = line
            .split_ascii_whitespace()
            .skip(1)
            .map(|v| v.parse::<u32>().expect("expected integer for size"));
        Ok((iter.next().unwrap(), iter.next().unwrap()))
    }

    pub fn write(&mut self, x: u32, y: u32, color: (u8, u8, u8)) -> io::Result<()> {
        let hex = format!("{:02x}{:02x}{:02x}", color.0, color.1, color.2);
        writeln!(self.write, "PX {} {} {}", x, y, hex)
    }
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

    let host = HOST;
    let (width, height) = Pixelflut::connect(host)
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

            std::thread::spawn(move || loop {
                let mut pixelflut =
                    Pixelflut::connect(host).expect("failed to connect to pixelflut on thread");
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
