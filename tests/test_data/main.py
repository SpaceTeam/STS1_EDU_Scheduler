import os
import sys

def main(queue_id: str):
    if "results" not in os.listdir():
        os.mkdir("results")
    if queue_id not in os.listdir("results"):
        os.mkdir(f"results/{queue_id}")

    if queue_id == "0001":
        with open(f"results/{queue_id}/res.txt", "w") as f:
            f.write("Some test results\nWith multiple lines\n")
    elif queue_id == "0002":
        while True:
            pass
    elif queue_id == "0003":
        from importlib import import_module


if __name__ == "__main__":
    main(sys.argv[1])
