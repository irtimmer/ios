use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use core::fmt::Write;
use core::future;

use futures_util::StreamExt;

use spin::RwLock;

use crate::block::Block;
use crate::block::mbr::Mbr;
use crate::drivers::block::virtio_blk::VirtioBlk;
use crate::drivers::i8042::KeyboardStream;
use crate::fs::FileSystem;
use crate::fs::vfat::VFat16;
use crate::process::{Process, Thread};
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
            "ls" => ls().await,
            "cat" => cat(args).await,
            "process" => process().await,
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

pub async fn ls() {
    let block = runtime().get::<VirtioBlk>().unwrap();

    let mbr = Mbr::new(block).await.unwrap();
    let partition = mbr.get_partition(0).await.unwrap();

    let fs = VFat16::new(Arc::new(partition)).await.unwrap();
    let mut entries = fs.listdir("/").await.unwrap();

    while let Some(entry) = entries.next().await {
        writeln!(runtime().console.lock(), "{:?}", entry).unwrap();
    }
}

pub async fn cat(args: &str) {
    let block = runtime().get::<VirtioBlk>().unwrap();

    let mbr = Mbr::new(block).await.unwrap();
    let partition = mbr.get_partition(0).await.unwrap();

    let fs = Arc::new(VFat16::new(Arc::new(partition)).await.unwrap());
    let entries = fs.listdir("/").await.unwrap();
    let entry = entries.filter(|e| future::ready(e.name == args)).next().await.unwrap();

    let mut file = fs.open_node(entry.inode, entry.length as u64).await.unwrap();
    let mut buf = [0u8; 512];
    let len = file.read(&mut buf).await.unwrap();
    runtime().console.lock().write_str(&String::from_utf8_lossy(&buf[0..len])).unwrap();
}

pub async fn process() {
    let mut process = Process::new();
    process.load();
    let process = Arc::new(RwLock::new(process));

    let thread = Thread::new(process);
    thread.activate();
}
