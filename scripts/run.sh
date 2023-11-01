#!/bin/sh

cargo build --bin ios && \
cargo build --bin bootloader && \
cargo build --bin prog1 --release && \
mkdir -p ./esp/efi/boot && \
cp ./target/x86_64-unknown-none/release/prog1 ./esp/prog1 && \
cp ./target/x86_64-unknown-uefi/debug/bootloader.efi ./esp/efi/boot/bootx64.efi && \
qemu-system-x86_64 --enable-kvm -m 512M -smp 2 \
    -drive if=pflash,format=raw,readonly=on,file=OVMF_CODE.fd \
    -drive if=pflash,format=raw,readonly=on,file=OVMF_VARS.fd \
    -drive if=none,id=boot,format=raw,file=fat:rw:esp \
    -device virtio-blk-pci,drive=boot \
    -device isa-debug-exit,iobase=0xf4,iosize=0x04 -serial stdio
