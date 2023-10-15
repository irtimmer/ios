use alloc::string::String;

use core::fmt::Write;

use futures_util::StreamExt;

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
        writeln!(runtime().console.lock(), "{}", input).unwrap();
    }
}
