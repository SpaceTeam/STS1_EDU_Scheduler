import os
import sys
import time


def main(queue_id: str):
    if "results" not in os.listdir():
        os.mkdir("results")

    if queue_id == "0":
        with open(f"results/{queue_id}", "w") as f:
            f.write("Some test results\nWith multiple lines\n")
    elif queue_id == "1":
        while True:
            pass
    elif queue_id == "2":
        with open(f"results/{queue_id}/res.txt", "w") as f:
            f.write("First Line\n")
            f.flush()
            time.sleep(1)
            f.write("Second Line\n")


if __name__ == "__main__":
    main(sys.argv[1])
