import traceback
from typing import Callable
from waveform_tools import WF_Device, COBC
from fabric import Connection


class EDU_Tests:
    def __init__(self) -> None:
        self.device = WF_Device()
        self._tests = []
        self._failures = []

    def prepare(self) -> None:
        self.device.connect()
        self.cobc = COBC(self.device, 0, 1, 2, 3, 4)
        self.ssh = Connection("edu")
        self.ssh.open()
        self._upload()

    def register(self, func: Callable) -> None:
        self._tests.append(func)

    def run(self) -> int:
        for t in self._tests:
            self._reset()
            print(f"Running test {t.__name__}...")

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
            self.ssh.run("rm data/* archives/*")
            self.ssh.sudo("./STS1_EDU_Scheduler", disown=True)            

    def _kill_scheduler(self):
        if self.ssh.run("ps -C STS1_EDU_Scheduler").exited is 0:
            self.ssh.sudo("ps -C STS1_EDU_Scheduler -o pid= | xargs kill")

    def _upload(self):
        self._kill_scheduler()
        self.ssh.put(local="./edu/STS1_EDU_Scheduler", remote="./scheduler/STS1_EDU_Scheduler")
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