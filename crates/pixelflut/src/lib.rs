use tokio::{
    io::{self, AsyncBufReadExt, AsyncWriteExt},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
};

pub struct Pixelflut {
    writer: OwnedWriteHalf,
    reader: io::BufReader<OwnedReadHalf>,
}

impl Pixelflut {
    pub async fn connect(host: &str) -> io::Result<Pixelflut> {
        let (reader, writer) = TcpStream::connect(host).await?.into_split();
        let reader = io::BufReader::new(reader);
        Ok(Self { writer, reader })
    }

    pub async fn size(&mut self) -> io::Result<(u32, u32)> {
        self.writer.write_all(b"SIZE\n").await?;

        let mut line = String::new();
        self.reader.read_line(&mut line).await?;

        let mut iter = line
            .split_ascii_whitespace()
            .skip(1)
            .map(|v| v.parse::<u32>().expect("expected integer for size"));
        Ok((iter.next().unwrap(), iter.next().unwrap()))
    }

    pub async fn read(&mut self, x: u32, y: u32) -> io::Result<(u8, u8, u8)> {
        self.writer
            .write_all(format!("PX {} {}\n", x, y).as_bytes())
            .await?;

        let mut line = String::new();
        self.reader.read_line(&mut line).await?;

        let colour_string = line
            .split_ascii_whitespace()
            .nth(3)
            .expect("expected colour at end of PX");

        Ok((
            u8::from_str_radix(&colour_string[0..2], 16).unwrap(),
            u8::from_str_radix(&colour_string[2..4], 16).unwrap(),
            u8::from_str_radix(&colour_string[4..6], 16).unwrap(),
        ))
    }

    pub async fn write(&mut self, x: u32, y: u32, color: (u8, u8, u8)) -> io::Result<()> {
        let hex = format!("{:02x}{:02x}{:02x}", color.0, color.1, color.2);
        self.writer
            .write_all(format!("PX {} {} {}\n", x, y, hex).as_bytes())
            .await?;
        Ok(())
    }
}
