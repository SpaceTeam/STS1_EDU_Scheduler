import os
import sys

if __name__ == "__main__":
    if "results" not in os.listdir():
        os.mkdir("results")
    if sys.argv[1] not in os.listdir("results"):
        os.mkdir(sys.argv[1])

    with open(f"results/{sys.argv[1]}/res.txt", "w") as f:
        f.writelines("""
        Some test results
        With multiple lines
        """)
