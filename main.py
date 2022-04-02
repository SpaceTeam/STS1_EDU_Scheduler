import threading
import time
import logging
from multiprocessing import Process, Queue
import os

import communication


def heartbeat() -> None:
    while True:
        logging.debug("Beat!")
        time.sleep(2)


if __name__ == '__main__':
    logging.basicConfig(level=logging.DEBUG, format='%(asctime)s:%(levelname)s %(message)s', datefmt="%H:%M:%S")

    paths = os.listdir()
    for name in ["data", "archives"]:
        if name not in paths:
            logging.info(f"Creating folder {name}")
            os.mkdir(name)

    logging.info("Starting heartbeat & communication process")
    threading.Thread(target=heartbeat, daemon=True).start()

    rx_queue = Queue()
    tx_queue = Queue()
    p = Process(target=communication.main_loop, args=(rx_queue, tx_queue))
    p.start()
    logging.info(f"{rx_queue.get()}")
    p.kill()
