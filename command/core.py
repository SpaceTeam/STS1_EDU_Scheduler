import os
import shutil
from zipfile import ZipFile, BadZipFile, LargeZipFile
import subprocess
from importlib import import_module
import time
import threading

from communication import COBC_CMD


class CommandHandler:
    is_program_running: bool
    last_program_killed: bool
    student_process: subprocess.Popen

    def __init__(self):
        self.is_program_running = False
        self.last_program_killed = False

    def dispatch_command(self, cmd: COBC_CMD, data_path: str) -> None:
        """
        This function dispatches a COBC command to the appropriate functions.

        :param cmd: Command type
        :param data: Data associated with the command
        """
        # TODO implement data preprocessing?
        raise NotImplementedError

    def store_archive(self, folder: str, zip_bytes: bytes) -> None:
        """
        This function stores the received bytes as a zipped file, the unzips and copies the python script to the
        appropriate location.

        :param folder: The name of the folder where the unzipped file should be placed
        :param zip_bytes: byte stream of a zip file
        """
        path = f"./archives/{folder}"
        if folder in os.listdir("./archives"):
            shutil.rmtree(path)
        os.mkdir(path)
        with open(f"{path}/tmp.zip", "wb") as file:
            file.write(zip_bytes)

        try:
            with ZipFile(f"{path}/tmp.zip") as zipf:
                zipf.extractall(path)
        except (BadZipFile, LargeZipFile, ValueError, NotImplementedError):
            pass  # TODO locked by COBC

        os.remove(f"{path}/tmp.zip")

    def execute_file(self, program: str, queue_id: str) -> None:
        """
        Executes a previously stored python script.

        :param program: The name of the program to execute
        :param queue_id: The id to pass to the program
        """
        if self.is_program_running:
            return  # TODO Error handling, locked by COBC

        self.last_program_killed = False
        self.is_program_running = True

        self.student_process = subprocess.Popen(["python", "main.py", queue_id], cwd=f"./archives/{program}/")
        threading.Thread(target=self.__supervisor()).start()

    PROCESS_TIMEOUT = 1  # Timeout in seconds TODO find useful value

    def __supervisor(self):
        step = CommandHandler.PROCESS_TIMEOUT/10
        for _ in range(10):
            if self.student_process.poll() is not None:
                break
            time.sleep(step)

        if self.student_process.poll() is None:
            self.student_process.kill()
            self.student_process.wait(0.01)
            self.last_program_killed = True
        # TODO else: set GPIO?
        self.is_program_running = False

    def stop_program(self) -> None:
        """
        Stops the execution of the currently running student program.
        """
        if self.is_program_running:
            self.student_process.kill()
            self.student_process.wait(0.01)
            self.last_program_killed = True
            self.is_program_running = False
        else:
            pass  # TODO not running error handling

    def return_results(self, program_id: str, queue_id: str) -> None:
        """
        Sends the results of an execution to the communcation module for transmission

        :param program_id: The programs result to return
        :param queue_id: The queue_id of the requested run
        """
        if not os.path.exists(f"./archives/{program_id}/results/{queue_id}"):
            # TODO error handling
            return
        shutil.make_archive(f"./data/{program_id}_{queue_id}", format="zip",
                            root_dir=f"./archives/{program_id}/results/", base_dir=queue_id)



    def list_files(self) -> None:
        """
        Sends the currently stored python scripts to the communication module for transmission
        """
        raise NotImplementedError

    def update_time(self, data: int) -> None:
        """
        Updates the EDU systems time.

        :param data: seconds since epoch
        """
        raise NotImplementedError
