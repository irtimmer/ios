# Iwan's OS

Iwan's OS is an toy operating system written in Rust.
It tries to use leverage modern interfaces without support for legacy harware interface like BIOS and PIC.

Kernel is only tested on Qemu on x86_64 with OVMF UEFI bios.

## Howto run
Copy OVMF_CODE.fd and OVMF_VARS.fd to root of this repository and run `./scripts/run.sh`

## Hardware support

### System
- x86_64
- UEFI
- ACPI
- SMP
- LAPIC
- IOAPIC

### Graphics support
- GOP framebuffer

### Input
- i8042 PS/2 keyboard

### Block devices
- virtio-blk-pci

### Partition table
- MBR

### Filesystems
- FAT16 (read-only)

## Crates

### kernel
This crate contains the kernel and drivers of Iwan's OS.

### bootloader
A very simple UEFI bootloader which will load the embedded kernel and setup higher half kernel memory mapping.

### userspace/prog1
Example helloworld userspace application that can be loaded with the command `process PROG1` in Iwan's OS.

## Credits

Iwan's OS is derived from following tutorials and software distributions:

- Philipp Oppermann's [series of blog posts][opp].
- Code from the [Hermit unikernel][hermit] project.
- Uses libraries from the [Rust OSDev][rustosdev] community.

[opp]: http://blog.phil-opp.com
[hermit]: http://hermit-os.org
[rustosdev]: https://rust-osdev.com

## License

This project, with exception of the `blog/content` folder, is licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  https://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
