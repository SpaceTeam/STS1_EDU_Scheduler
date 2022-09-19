import logging
from AD_UART import AD_UART

if __name__ == "__main__":
    logging.basicConfig(level=logging.DEBUG)
    uart = AD_UART(115200, 0, 1, 8, 1, 1)
    uart.send(b"\x00\x01\x02")
    echo = uart.receive(3, 1)
    if echo == b"\x00\x01\x02":
        logging.info("Success")
    else:
        logging.info(f"Failed with\n{echo}")
