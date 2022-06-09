use mini_redis::{client, Result};
use tokio::sync::{
    mpsc,
    oneshot,
};
use bytes::Bytes;

#[derive(Debug)]
enum Command {
    Get {
        key: String,
        resp: Responder<Option<Bytes>>,
    },
    Set {
        key: String,
        val: Bytes,
        resp: Responder<()>,
    }
}

type Responder<T> = oneshot::Sender<Result<T>>;

#[tokio::main]
pub async fn main() -> Result<()> {
    let (tx, mut rx) = mpsc::channel(32);

    let manager = tokio::spawn(async move {
        // mini-redis アドレスへのコネクションを開く
        let mut client = client::connect("127.0.0.1:6379").await.unwrap();

        while let Some(cmd) = rx.recv().await {
            use Command::*;

            match cmd {
                Get { key, resp } => {
                    let res = client.get(&key).await;
                    // エラーは無視する
                    let _ = resp.send(res);
                },
                Set { key, val, resp } => {
                    let res = client.set(&key, val).await;
                    // エラーは無視する
                    let _ = resp.send(res);
                },
            }
        }
    });

    let tx2 = tx.clone();

    let t1 = tokio::spawn(async move {
        let (resp_tx, resp_rx) = oneshot::channel();
        let cmd = Command::Get {
            key: "foo".to_string(),
            resp: resp_tx,
        };

        // GET リクエストを送信
        tx.send(cmd).await.unwrap();

        // レスポンスが来るのを待つ
        let res = resp_rx.await;
        println!("GOT (Get) = {:?}", res);
    });

    let t2 = tokio::spawn(async move {
        let (resp_tx, resp_rx) = oneshot::channel();
        let cmd = Command::Set {
            key: "foo".to_string(),
            val: "bar".into(),
            resp: resp_tx,
        };

        // SET リクエストを送信
        tx2.send(cmd).await.unwrap();

        // レスポンスが来るのを待つ
        let res = resp_rx.await;
        println!("GOT (Set) = {:?}", res);
    });

    t1.await.unwrap();
    t2.await.unwrap();
    manager.await.unwrap();

    Ok(())
}
