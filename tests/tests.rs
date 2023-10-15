/// This directory contains tests that only use command::handle_command(). This makes them ideal for debugging and finegrained tests
mod software_tests;

/// This directory contains tests that run the actual compiled binaries and uses socat to create a simulated serial port.
mod simulation;
