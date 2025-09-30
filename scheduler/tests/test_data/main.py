import os
import sys
import time
import socket
from pathlib import Path


def main(queue_id: str):
    print(f"Hello from Py with Queue ID {queue_id}")

    if queue_id not in os.listdir():
        os.mkdir(queue_id)

    if queue_id == "0":
        with open(f"{queue_id}/result", "w") as f:
            f.write("Some test results\nWith multiple lines\n")
    elif queue_id == "1":
        while True:
            pass
    elif queue_id == "2":
        with open(f"{queue_id}/result", "w") as f:
            f.write("First Line\n")
            f.flush()
            time.sleep(1)
            f.write("Second Line\n")
    elif queue_id == "3":
        with open(f"{queue_id}/result", "wb") as f:
            f.write(b"\xde\xad")
    elif queue_id == "4":
        raise EnvironmentError
    elif queue_id == "5":
        with open(f"{queue_id}/result", "wb") as f:
            for _ in range(1700000):
                f.write(b"\xfe")
    elif queue_id == "6":
        socket_path = "/tmp/scheduler_socket" if Path("/tmp/scheduler_socket").exists() else "/tmp/STS1_EDU_Scheduler_SIM_dosimeter_python"
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
            client.connect(socket_path)
            client.send(b"dosimeter/on\n")
            client.close()


if __name__ == "__main__":
    main(sys.argv[1])
