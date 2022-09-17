/// This module seeks to simulate the connection between the `Raspi` and the `COBC` without actually needing the hardware
/// This module, therefore, does not contemplate UART Errors and those have to be forced into the test functions to 
/// observe the behaviour of the Raspi when facing those terrible cincumstances.\
/// 
/// Sorry not sorry for the use and abuse of OOP.

use core::time;
use std::{
    time::Duration, 
    thread, sync::{Arc, RwLock, Mutex, MutexGuard, RwLockWriteGuard, mpsc, PoisonError}, 
    io::Bytes, 
    fmt::write, result};

use {
    STS1_EDU_Scheduler::command::return_result,
    STS1_EDU_Scheduler::communication::{ComResult, CommunicationHandle, CommunicationError, CSBIPacket},
};

use log::warn;


//Constants used for the real thing
const DATA_BITS: u8 = 8; //For the UART config, not used here
const STOP_BITS: u8 = 1; //Same thing
const ALLOWED_SEND_RETRIES: u8 = 3; //Same thing
const MAX_READ_TIMEOUT:Duration = Duration::from_secs(25);

//Some consts for testing
const SEND_DURATION: Duration = Duration::from_millis(50);

pub type UARTSimHandleRef = Arc<RwLock<UARTSimHandle>>;
pub type COBCSimRef = Arc<RwLock<COBCSim>>;

/// The simulated UART for the Raspi. It implements all the functions of `CommunicationHandle` but
/// without the hardware.
/// 
/// ## Components
/// * `cobc`: is the simulated COBC with which is shall communicate.
pub struct UARTSimHandle {
    pub cobc: Option<COBCSimRef>,
    pub tx_buffer: Arc<RwLock<Vec<u8>>>,
    pub rx_buffer: Arc<RwLock<Vec<u8>>>,
    pub write_block: bool,
    baudrate: u32
}

impl UARTSimHandle {
    pub fn new(baudrate: u32) -> Self {
        return UARTSimHandle {
           cobc: None,
           tx_buffer: Arc::new(RwLock::new(Vec::new())),
           rx_buffer: Arc::new(RwLock::new(Vec::new())),
           write_block: true,
           baudrate: baudrate
        };
    }

    /// Adds the `COBC` component to the `Raspi`.
    /// It doesn't connect the other way around. Use the `COBCSim`'s `connect_raspi_to_cobc` for that.
    pub fn connect_cobc_to_raspi(&mut self, cobc: &COBCSimRef) {
        self.cobc = Some(cobc.clone());
    }
    

    pub fn print_name(&self) {
        println!("I'm the raspi");
    }
}

impl CommunicationHandle for UARTSimHandle {

    /// Sends `bytes` via simulated UART. For simplifaction purposes,
    /// it writes directly into the `COBC`'s `rx_buffer`.
    /// 
    /// Use the `COBC`'s `receive` to actually read its rx_buffer
    fn send(&mut self, bytes: Vec<u8>) -> ComResult<()> {

        let mut write_buffer = self.tx_buffer.write().unwrap();
        *write_buffer = bytes.clone();
        let inter_byte_duration: Duration = Duration::from_nanos(1_000_000 / (self.baudrate * 8) as u64);

        //Acquire COBC's rx_buffer;
        let cobc_rx_ref: Arc<RwLock<Vec<u8>>>;
        let mut cobc_rx_buffer: RwLockWriteGuard<Vec<u8>>;
        {
            let cobc_temp = self.cobc.as_ref().unwrap().read().unwrap(); //dies with the scope
            cobc_rx_ref = cobc_temp.rx_buffer.clone();
        }
        cobc_rx_buffer = cobc_rx_ref.write().unwrap();

        // Start bit
        thread::sleep(inter_byte_duration);

        for i in 0..write_buffer.len() {
            cobc_rx_buffer.push(write_buffer[i]);
            thread::sleep(inter_byte_duration);
        }
        return Ok(());
    }

    fn receive(&mut self, byte_count: u16, timeout: &std::time::Duration) -> ComResult<Vec<u8>> {
        let (ch_sender, ch_receiver) = mpsc::channel::<ComResult<Vec<u8>>>();
        let rx_ref = self.rx_buffer.clone(); //clone Arc and give it to Mr. New Thread.

        thread::spawn(move || {
            match rx_ref.read() { //grabs the Arc
                Ok(byte_vec) => {ch_sender.send(Ok(byte_vec.clone())).unwrap();},
                Err(_e) => {ch_sender.send(Err(CommunicationError::InterfaceError)).unwrap();}
            }
        });

        thread::sleep(*timeout);
        if let Ok(read_result) = ch_receiver.try_recv() {
            let ret = read_result?;
            return Ok(ret);
        } else {
            return Err(CommunicationError::TimeoutError);
        }
    }

}

/// This is the COBC (Sim), which we will use to simulate the communication with the raspi.
/// ## Components
/// * `raspi`: The UARTSimHandle to which it will send data and where it will get data from.
pub struct COBCSim 
{
    pub raspi: Option<UARTSimHandleRef>,
    pub tx_buffer: Arc<RwLock<Vec<u8>>>,
    pub rx_buffer: Arc<RwLock<Vec<u8>>>,
    pub write_block: bool,
    baudrate: u32
}

impl COBCSim {
    pub fn new(baudrate: u32) -> Self {
        return COBCSim {
            raspi: None,
            tx_buffer: Arc::new(RwLock::new(Vec::new())),
            rx_buffer: Arc::new(RwLock::new(Vec::new())),
            write_block: true,
            baudrate: baudrate
        };
    }

    /// Connects the `Raspi` component to the `COBC`.
    /// It doesn't connect the other way around. Use `UARTSimHandle`'s `connect_cobc_to_raspi` for that.
    pub fn connect_raspi_to_cobc(&mut self, raspi: &UARTSimHandleRef) {
        self.raspi = Some(raspi.clone());
    }

    pub fn print_name(&self) {
        println!("I'm the COBC");
    }
}

impl CommunicationHandle for COBCSim {

    /// Sends `bytes` via simulated UART. For simplifaction purposes,
    /// it writes directly into the `Raspi`'s `rx_buffer`.
    /// 
    /// Use the `Raspi`'s `receive` to actually read its rx_buffer
    fn send(&mut self, bytes: Vec<u8>) -> ComResult<()> {

        let mut write_buffer = self.tx_buffer.write().unwrap();
        *write_buffer = bytes.clone();
        let inter_byte_duration: Duration = Duration::from_nanos(1_000_000 / (self.baudrate * 8) as u64);

        //Acquire Raspi's rx_buffer;
        let raspi_rx_ref: Arc<RwLock<Vec<u8>>>;
        let mut raspi_rx_buffer: RwLockWriteGuard<Vec<u8>>;
        {
            let cobc_temp = self.raspi.as_ref().unwrap().read().unwrap(); //dies with the scope
            raspi_rx_ref = cobc_temp.rx_buffer.clone();
        }
        raspi_rx_buffer = raspi_rx_ref.write().unwrap();

        // Start bit
        thread::sleep(inter_byte_duration);

        for i in 0..write_buffer.len() {
            raspi_rx_buffer.push(write_buffer[i]);
            thread::sleep(inter_byte_duration);
        }
        return Ok(());
    }

    fn receive(&mut self, byte_count: u16, timeout: &std::time::Duration) -> ComResult<Vec<u8>> {
        let (ch_sender, ch_receiver) = mpsc::channel::<ComResult<Vec<u8>>>();
        let rx_ref = self.rx_buffer.clone(); //clone Arc and give it to Mr. New Thread.

        thread::spawn(move || {
            match rx_ref.read() { //grabs the Arc
                Ok(byte_vec) => {ch_sender.send(Ok(byte_vec.clone())).unwrap();},
                Err(_e) => {ch_sender.send(Err(CommunicationError::InterfaceError)).unwrap();}
            }
        });

        thread::sleep(*timeout);
        if let Ok(read_result) = ch_receiver.try_recv() {
            let ret = read_result?;
            return Ok(ret);
        } else {
            return Err(CommunicationError::TimeoutError);
        }
    }
}


#[test]
fn sim_raspi_send_cobc_receive_bytes() -> Result<(), Box<dyn std::error::Error>> {
    
    let raspi = Arc::new(RwLock::new(UARTSimHandle::new(115200)));
    let cobc = Arc::new(RwLock::new(COBCSim::new(115200)));
    // Connect them mfs
    {
        raspi.write().unwrap().cobc = Some(cobc.clone());
        cobc.write().unwrap().raspi = Some(raspi.clone());
    }

    let timeout: Duration = Duration::from_nanos(2);

    let _ = raspi.write().unwrap().send(vec![1, 2, 3, 4]);
    let res = cobc.write().unwrap().receive(4, &timeout);
    assert_eq!(res.unwrap(), vec![1, 2, 3, 4]);

    return Ok(());
}

#[test]
fn sim_cobc_send_raspi_receive_bytes() -> Result<(), Box<dyn std::error::Error>> {
    let raspi = Arc::new(RwLock::new(UARTSimHandle::new(115200)));
    let cobc = Arc::new(RwLock::new(COBCSim::new(115200)));
    // Connect them mfs
    {
        raspi.write().unwrap().cobc = Some(cobc.clone());
        cobc.write().unwrap().raspi = Some(raspi.clone());
    }

    let timeout: Duration = Duration::from_nanos(2);

    let _ = cobc.write().unwrap().send(vec![1, 2, 3, 4]);
    let res = raspi.write().unwrap().receive(4, &timeout);

    assert_eq!(res.unwrap(), vec![1, 2, 3, 4]);

    return Ok(());
}
