import logging
from AD_UART import AD_UART

if __name__ == "__main__":
    logging.basicConfig(level=logging.DEBUG)
    uart = AD_UART(115200, 0, 1, 8, 1, 1)
