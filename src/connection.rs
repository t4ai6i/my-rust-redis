use tokio::net::TcpStream;
use mini_redis::{Frame, Result};

struct Connection {
    stream: TcpStream,
}

impl Connection {

    /// コネクションからフレームを読み取る
    ///
    /// EOF に達したら `None` を返す
    pub async fn read_frame(&mut self) -> Result<Option<Frame>> {
        unimplemented!()
    }

    pub async fn write_frame(&mut self, frame: &Frame) -> Result<()> {
        unimplemented!()
    }
}