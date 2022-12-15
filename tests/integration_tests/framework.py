import traceback
import logging
from typing import Callable
from waveform_tools import WF_Device, COBC
from fabric import Connection


class EDU_Tests:
    def __init__(self) -> None:
        self.device = WF_Device()
        self._tests = []
        self._failures = []

    def prepare(self) -> None:
        logging.info("Connecting to logic analyzer...")
        self.device.connect()
        self.device.reset()
        self.cobc = COBC(self.device, 3, 2, 5, 12, 4)
        logging.info("Connecting to EDU...")
        self.ssh = Connection("edu")
        self.ssh.open()
        self._upload()

    def register(self, func: Callable) -> None:
        self._tests.append(func)

    def run(self) -> int:
        for t in self._tests:
            self._reset()
            print(f"{bcolors.BOLD}Running test {t.__name__}...{bcolors.ENDC}")

            try:
                t(self.cobc)
            except Exception:
                traceback.print_exc()
                print(f"{bcolors.FAIL}Failure{bcolors.ENDC}\n")
                self._failures.append(t.__name__)
            else:
                print(f"{bcolors.OKGREEN}Successful{bcolors.ENDC}\n")

        if len(self._failures) == 0:
            print(f"{bcolors.OKGREEN}All tests passed successfully{bcolors.ENDC}")
        else:
            print(f"{bcolors.FAIL}The following tests failed:{bcolors.ENDC}")
            for f in self._failures:
                print(f"\t{f}")
        return len(self._failures)

    def _reset(self):
        self._kill_scheduler()
        with self.ssh.cd("./scheduler"):
            self.ssh.run("rm -rf data/* archives/*", warn=True)
            self.ssh.run("./STS1_EDU_Scheduler", disown=True)            

    def _kill_scheduler(self):
        if self.ssh.run("ps -C STS1_EDU_Scheduler", warn=True).exited == 0:
            logging.info("Scheduler is already running, killing...")
            self.ssh.run("ps -C STS1_EDU_Scheduler -o pid= | xargs kill")

    def _upload(self):
        logging.info("Uploading scheduler from flatsat...")
        self._kill_scheduler()
        self.ssh.put(local="C:/Users/ssh/edu/STS1_EDU_Scheduler", remote="./scheduler/STS1_EDU_Scheduler")
        self.ssh.run("chmod +x ./scheduler/STS1_EDU_Scheduler")


class bcolors:
    HEADER = '\033[95m'
    OKBLUE = '\033[94m'
    OKCYAN = '\033[96m'
    OKGREEN = '\033[92m'
    WARNING = '\033[93m'
    FAIL = '\033[91m'
    ENDC = '\033[0m'
    BOLD = '\033[1m'
    UNDERLINE = '\033[4m'
