from ctypes import *
import logging
import time
import sys

class AD_UART:
    def __init__(self, baudrate: int, tx_pin: int, rx_pin: int, bits_per_byte: int, parity: int, stop_length: int):
        if sys.platform.startswith("win"):
            self._dwf = cdll.LoadLibrary("dwf.dll")
        elif sys.platform.startswith("darwin"):
            self._dwf = cdll.LoadLibrary("/Library/Frameworks/dwf.framework/dwf")
        else:
            self._dwf = cdll.LoadLibrary("libdwf.so")

        logging.debug("Aquiring device...")
        self._hdwf = c_int()
        self._dwf.FDwfDeviceOpen(c_int(-1), byref(self._hdwf))
        if self._hdwf.value == 0:
            logging.error("failed to open device")
            szerr = create_string_buffer(512)
            self._dwf.FDwfGetLastErrorMsg(szerr)
            logging.error(str(szerr.value))
            raise ConnectionError()

        logging.debug("Configuring UART...")
        self._dwf.FDwfDigitalUartRateSet(self._hdwf, c_double(baudrate))
        self._dwf.FDwfDigitalUartTxSet(self._hdwf, c_int(tx_pin)) 
        self._dwf.FDwfDigitalUartRxSet(self._hdwf, c_int(rx_pin)) 
        self._dwf.FDwfDigitalUartBitsSet(self._hdwf, c_int(bits_per_byte))
        self._dwf.FDwfDigitalUartParitySet(self._hdwf, c_int(parity))
        self._dwf.FDwfDigitalUartStopSet(self._hdwf, c_double(stop_length))

        self._rx_count = c_int(0)
        self._rx_parity = c_int(0)
        self._dwf.FDwfDigitalUartTx(self._hdwf, None, c_int(0)) # initialize TX, drive with idle level
        self._dwf.FDwfDigitalUartRx(self._hdwf, None, c_int(0), byref(self._rx_count), byref(self._rx_parity)) # initialize RX reception

    def send(self, data: bytearray):
        logging.debug(f"Sending {bytearray}...")
        rgTX = create_string_buffer(data)
        self._dwf.FDwfDigitalUartTx(self._hdwf, rgTX, c_int(sizeof(rgTX)-1)) # send data, cut off \0

    def receive(self, n_bytes: int, timeout: float) -> bytearray:
        rgRX = create_string_buffer(n_bytes)
        buffer = bytearray()
        end_time = time.perf_counter() + timeout

        while time.perf_counter < end_time:
            self._dwf.FDwfDigitalUartRx(self._hdwf, rgRX, c_int(sizeof(rgRX)), byref(self._rx_count), byref(self._rx_parity))
            if self._rx_count.value > 0:
                buffer.extend(rgRX.raw[:self._rx_count.value])
            if self._rx_parity.value != 0:
                raise ParityError(f"Received {rgRX.raw}")

        if len(buffer) < n_bytes:
            raise TimeoutError()

        return buffer


class ParityError(Exception):
    pass