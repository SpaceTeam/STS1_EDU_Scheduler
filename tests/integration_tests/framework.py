import logging
import traceback
from typing import Callable, Literal
from waveform_tools import WF_Device, COBC


class EDU_Tests:
    def __init__(self) -> None:
        self.device = WF_Device()
        self._full_tests = []
        self._quick_tests = []
        self._failures = []

    def prepare(self) -> None:
        self.device.connect()
        self.cobc = COBC(self.device, 0, 1, 2, 3, 4)

    def register(self, func: Callable, type: Literal['quick', 'full reset']) -> None:
        self._tests.append(func)

    def run(self, type: Literal['all', 'quick', 'full']) -> None:
        tests = []
        if type == 'all' or type == 'quick':
            tests.append(self._quick_tests)
        if type == 'all' or type == 'full':
            tests.append(self._full_tests)

        for t in tests:
            print(f"Running test {t.__name__}...")
            try:
                t(self.cobc)
            except Exception:
                traceback.print_exc()
                print(f"{bcolors.FAIL}Failure{bcolors.ENDC}")
                self._failures.append(t.__name__)
            else:
                print(f"{bcolors.OKGREEN}Successful{bcolors.ENDC}")

        if len(self._failures) == 0:
            print(f"{bcolors.OKGREEN}All tests passed successfully{bcolors.ENDC}")
        else:
            print(f"{bcolors.FAIL}The following tests failed:{bcolors.ENDC}")
            for f in self._failures:
                print(f"\t{f}")

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
