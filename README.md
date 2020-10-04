# PouetPouet

The firmware is [Keyberon](https://github.com/TeXitoi/keyberon), a
pure rust firmware.

## Features

 * 60 keys, using Cherry MX switches, only 1U keycaps;
 * USB-C connector;
 * 1 STM32F072 MCU, with hardware USB DFU bootloader and crystal less USB;
 * Only onboard SMD component (except for the switches).

## Inspiration

 * [Keyseebee](https://github.com/TeXitoi/keyseebee) for being the «I will change it quickly and have something ready in an hour» base project (even if I ended up redoing most of the hardware design).
 * [Steamvan](https://github.com/jmdaly/steamvan) for some KiCad design ideas;
 * [help-14](https://github.com/help-14/mechanical-keyboard) for making a nice list of existing keyboard;
 * [Masterzen](http://www.masterzen.fr/2020/05/03/designing-a-keyboard-part-1/) and many others for writing online tutorials for newbies like me.

## Bill Of Materials

|Item                                                                      |Package|Qty|Remarks                                |Price |
|--------------------------------------------------------------------------|-------|--:|---------------------------------------|-----:|


## Compiling and flashing

Install the complete toolchain and utils:

```shell
curl https://sh.rustup.rs -sSf | sh
rustup target add thumbv6m-none-eabi
rustup component add llvm-tools-preview
cargo install cargo-binutils
sudo apt-get install dfu-util
```

Compile:

```shell
cd firmware
cargo objcopy --bin pouetpouet --release -- -O binary pouetpouet.bin
```

To flash using dfu-util, first put the board in dfu mode by pressing
BOOT, pressing and releasing RESET and releasing BOOT. Then:

```shell
dfu-util -d 0483:df11 -a 0 -s 0x08000000:leave -D pouetpouet.bin
```

The fist time, if the write fail, your flash might be protected. To
unprotect:

```shell
dfu-util -d 0483:df11 -a 0 -s 0x08000000:force:unprotect -D pouetpouet.bin
```
