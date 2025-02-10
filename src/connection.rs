use bytes::{Buf, BytesMut};
use mini_redis::frame::Error::Incomplete;
use mini_redis::{Frame, Result};
use std::io::Cursor;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
use tokio::net::TcpStream;

struct Connection {
    stream: BufWriter<TcpStream>,
    buffer: BytesMut,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            stream: BufWriter::new(stream),
            // 4KB のキャパシティをもつバッファを確保する
            buffer: BytesMut::with_capacity(4096),
        }
    }

    /// コネクションからフレームを読み取る
    ///
    /// EOF に達したら `None` を返す
    pub async fn read_frame(&mut self) -> Result<Option<Frame>> {
        loop {
            // バッファされたデータからフレームをパースすることを試みる
            // 十分な量のデータがバッファに蓄えられていたら、ここでフレームを return する
            if let Some(frame) = self.parse_frame()? {
                return Ok(Some(frame));
            }

            // バッファデータが足りなかった場合
            // ソケットからさらにデータを読み取ることを試みる
            //
            // 成功した場合、バイト数が返ってくる
            // `0` は "ストリームの終わり" を意味する
            if 0 == self.stream.read_buf(&mut self.buffer).await? {
                // 相手側がコネクションを閉じた。
                // きれいにシャットダウンするため、read バッファのデータが空になる
                // ようにしなければならない。もしデータが残っているなら、それは
                // 相手がフレームを送信している途中でソケットを閉じたということを意味する
                return if self.buffer.is_empty() {
                    Ok(None)
                } else {
                    Err("connection reset by peer".into())
                };
            }
        }
    }

    fn parse_frame(&mut self) -> Result<Option<Frame>> {
        // `Buf` 型を作る
        let mut buf = Cursor::new(&self.buffer[..]);

        // フレーム全体が取得可能かどうかチェックする
        match Frame::check(&mut buf) {
            Ok(_) => {
                // フレームのバイト長を取得
                let len = buf.position() as usize;

                // `parse` を呼び出すため、内部カーソルをリセットする
                buf.set_position(0);

                // フレームをパースする
                let frame = Frame::parse(&mut buf)?;

                // フレームをパースする
                self.buffer.advance(len);

                // 呼び出し側にフレームを返す
                Ok(Some(frame))
            }
            // 十分な量のデータがバッファされていなかった場合
            Err(Incomplete) => Ok(None),
            // エラーが発生した場合
            Err(e) => Err(e.into()),
        }
    }

    pub async fn write_frame(&mut self, frame: &Frame) -> Result<()> {
        match frame {
            Frame::Simple(val) => {
                self.stream.write_u8(b'+').await?;
                self.stream.write_all(val.as_bytes()).await?;
                self.stream.write_all(b"\r\n").await?;
            }
            Frame::Error(val) => {
                self.stream.write_u8(b'-').await?;
                self.stream.write_all(val.as_bytes()).await?;
                self.stream.write_all(b"\r\n").await?;
            }
            Frame::Integer(val) => {
                self.stream.write_u8(b':').await?;
                self.write_decimal(*val).await?;
            }
            Frame::Bulk(val) => {
                let len = val.len();

                self.stream.write_u8(b'$').await?;
                self.write_decimal(len as u64).await?;
                self.stream.write_all(val).await?;
                self.stream.write_all(b"\r\n").await?;
            }
            Frame::Null => self.stream.write_all(b"$-1\r\n").await?,
            Frame::Array(_) => unimplemented!(),
        }

        self.stream.flush().await?;

        Ok(())
    }

    async fn write_decimal(&mut self, val: u64) -> Result<()> {
        // Convert the value to a string
        let mut buf = [0u8; 12];
        let mut buf = Cursor::new(&mut buf[..]);
        write!(&mut buf, "{}", val)?;

        let pos = buf.position() as usize;
        self.stream.write_all(&buf.get_ref()[..pos]).await?;
        self.stream.write_all(b"\r\n").await?;

        Ok(())
    }
}
