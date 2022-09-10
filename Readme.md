# How to setup compilation
Compiling rust is fortunately quite simple. First install rustup (see [rustup.rs](rustup.rs) for more info)

`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`

You now have `rustup` (installing/updating/modifying of the toolchain) and `cargo` (project manager) installed.

You can now `cd` into the STS1_EDU_Scheduler directory and use

* `cargo build` to compile the thing (first time takes long)
* `cargo test` compile and run all tests

## Compiling for the Raspberry Pi
To compile for the compute module, there are two ways

### With docker already installed
Install the cross tool

`cargo install cross --git https://github.com/cross-rs/cross`

Compile (make sure docker is running)

`cross build --release --target armv7-unknown-linux-gnueabihf`

The executable can be found in `./target/armv7-unknown-linux-gnueabihf/release/STS1_EDU_Scheduler`

### Without docker
Install the target

`rustup add target armv7-unknown-linux-gnueabihf`

Install the toolchain

`sudo apt install gcc-arm-linux-gnueabihf`

Compile

`cargo build --release --target=armv7-unknown-linux-gnueabihf`

The executable can be found in `./target/armv7-unknown-linux-gnueabihf/release/STS1_EDU_Scheduler`

# How to actually test the thing

## Setting up the logic analyzer
Install WaveForms on your laptop ([digilent.com](https://digilent.com/shop/software/digilent-waveforms/download))

Hookup the analog discovery thingy to power and your laptop.

Connect cable 0 to Pin 15 (UART TX) of the dev board and cable 1 to Pin 14 (UART RX).

Open WaveForms and select the `Protocols` thing. Select the `UART` tab.

Set the baudrate to 115200.

Switch to the `Send & Receive` tab and enable Receiving.


## Running the scheduler

Hook up the blue dev board to a monitor/keyboard/mouse. Use a USB stick to transfer the aforementioned executable to the Desktop (overwrite existing). Open a terminal and enter

`cd ~/Desktop`

`chmod +x STS1_EDU_Scheduler`

`sudo ./STS1_EDU_Scheduler`

(su password is `raspberry`)


## Sending packets to the EDU

### Creating pack files
In order to wrap data into the CEP Packets, write stuff in `tests/raspberry_tests.rs` (examples provided).

Then use `make packs` to build them.

### Sending pack files
In WaveForms `Send & Receive` tab, click on the `send file` icon next to the send button.

Change the File type from `Text File (*.*)` to `Binary File (*.*)`!

Select the appropriate pack file