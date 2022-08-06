import os
import sys
import time


def main(queue_id: str):
    print(f"Hello from Py with Queue ID {queue_id}")
    
    if "results" not in os.listdir():
        os.mkdir("results")

    if queue_id == "0":
        with open(f"results/{queue_id}", "w") as f:
            f.write("Some test results\nWith multiple lines\n")
    elif queue_id == "1":
        while True:
            pass
    elif queue_id == "2":
        with open(f"results/{queue_id}", "w") as f:
            f.write("First Line\n")
            f.flush()
            time.sleep(1)
            f.write("Second Line\n")
    elif queue_id == "3":
        with open(f"results/{queue_id}", "wb") as f:
            f.write(b"0xde0xad")
    elif queue_id == "4":
        raise EnvironmentError


if __name__ == "__main__":
    main(sys.argv[1])
