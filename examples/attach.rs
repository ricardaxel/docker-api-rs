use docker_api::{conn::TtyChunk, Docker};
use futures::StreamExt;
use std::{env, str};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let docker = Docker::new("tcp://127.0.0.1:80")?;
    let id = env::args()
        .nth(1)
        .expect("You need to specify a container id");

    let tty_multiplexer = docker.containers().get(&id).attach().await?;

    let (mut reader, _writer) = tty_multiplexer.split();

    while let Some(tty_result) = reader.next().await {
        match tty_result {
            Ok(chunk) => print_chunk(chunk),
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    Ok(())
}

fn print_chunk(chunk: TtyChunk) {
    match chunk {
        TtyChunk::StdOut(bytes) => {
            println!("Stdout: {}", str::from_utf8(&bytes).unwrap_or_default())
        }
        TtyChunk::StdErr(bytes) => {
            eprintln!("Stdout: {}", str::from_utf8(&bytes).unwrap_or_default())
        }
        TtyChunk::StdIn(_) => unreachable!(),
    }
}
