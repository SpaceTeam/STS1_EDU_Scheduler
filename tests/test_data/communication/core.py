import enum
import time
from multiprocessing import Queue


class COBC_CMD(enum.Enum):
    NOP = 0
    STORE_ARCHIVE = 1
    EXECUTE_FILE = 2
    STOP_FILE = 3
    SEND_RESULTS = 4
    LIST_FILES = 5
    UPDATE_TIME = 6


def main_loop(rx_queue: Queue, tx_queue: Queue):
    # TODO
    while True:
        time.sleep(2)
        rx_queue.put((COBC_CMD.NOP, None), block=True)
