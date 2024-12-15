use tokio::io;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;

async fn main() -> io::Result<()> {
    let mut listener = TcpListener::bind("127.0.0.1:6142").await.unwrap();

    loop {
        let (mut socket, _) = listener.accept().await?;

        tokio::spawn(async move {
            let mut buf = vec![0; 1024];

            loop {
                match socket.read(&mut buf).await {
                    // `Ok(0)` が返ってきたらリモート側が閉じられたことを意味する
                    Ok(0) => return,
                    Ok(n) => {
                        // データをソケットへコピーする
                        if socket.write_all(&buf[..n]).await.is_err() {
                            // 予期しないソケットエラーが発生した場合
                            // ここで何かできることはさほどないので、処理を停止する
                            return;
                        }
                    },
                    Err(_) => {
                        // 予期しないソケットエラーが発生した場合
                        // ここで何かできることはさほどないので、処理を停止する
                        return;
                    }
                }
            }
        });
    }
}