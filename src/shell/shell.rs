use alloc::string::String;
use alloc::vec::Vec;

use core::fmt::Write;

use futures_util::StreamExt;

use crate::block::Block;
use crate::block::mbr::Mbr;
use crate::drivers::block::virtio_blk::VirtioBlk;
use crate::drivers::i8042::KeyboardStream;
use crate::runtime::runtime;

async fn read_line(kbd: &mut KeyboardStream<'_>) -> String {
    let mut input = String::with_capacity(16);
    while let Some(character) = kbd.next().await {
        runtime().console.lock().write_char(character).unwrap();

        if character == '\n' {
            break;
        }
        input.push(character);
    }
    input
}

pub async fn ios_shell() {
    let mut stream = runtime().keyboard.stream();

    loop {
        runtime().console.lock().write_str("> ").unwrap();
        let input = read_line(&mut stream).await;
        let (cmd, args) = input.split_once(' ').unwrap_or((&input, ""));
        match cmd {
            "echo" => writeln!(runtime().console.lock(), "{}", args).unwrap(),
            "read" => read(args).await,
            "part" => part().await,
            _ => writeln!(runtime().console.lock(), "Command '{}' not found", cmd).unwrap(),
        }
    }
}

pub async fn read(args: &str) {
    let args: Vec<&str> = args.split(' ').collect();

    let block = runtime().get::<VirtioBlk>().unwrap();
    let mut buf = [1u8; 512];
    block.read(buf.as_mut_slice(), args[0].parse().unwrap()).await.unwrap();

    writeln!(runtime().console.lock(), "Return: {:x?}", &buf).unwrap();
}

pub async fn part() {
    let block = runtime().get::<VirtioBlk>().unwrap();

    let mbr = Mbr::new(block).await.unwrap();
    mbr.partitions().iter().enumerate().for_each({
        |(i, p)| writeln!(runtime().console.lock(), "Partition {}: {:?}", i, p).unwrap()
    })
}
