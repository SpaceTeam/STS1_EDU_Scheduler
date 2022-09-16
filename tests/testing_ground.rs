/// This is the testing ground where we will "simulate" our components to bypass having the actual hardware to test our functions.
/// in "uart_sim.rs" we have an alterante version of the UART for the Raspi, one that doesn't use an actual raspi, but rather conneects
/// to our virtual COBC, which is also defined there. The COBC is only there to send and receive UART Data. Any other function the
/// actual COBC is going to perform is irrelevant for the tests and are therefore not simulated here.

mod uart_sim;

use std::sync::{Arc, Mutex, RwLock};
use uart_sim::{COBCSim, COBCSimRef, UARTSimHandle, UARTSimHandleRef};

const BAUDRATE: u32 = 115200;

#[cfg(feature = "nohw")]
mod tests {

    #[test]
    fn test_names() {
        

    }
}
