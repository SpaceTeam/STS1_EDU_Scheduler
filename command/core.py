import logging
from typing import Any
import os
import shutil
from zipfile import ZipFile

from communication import COBC_CMD


def dispatch_command(cmd: COBC_CMD, data_path: str) -> None:
    """
    This function dispatches a COBC command to the appropriate functions.

    :param cmd: Command type
    :param data: Data associated with the command
    """
    # TODO implement data preprocessing?
    raise NotImplementedError


def store_archive(folder: str, zip: bytes) -> None:
    """
    This function stores the received bytes as a zipped file, the unzips and copies the python script to the
    appropriate location.

    :param path: The name of the folder where the unzipped file should be placed
    :param data: byte stream of a zip file
    """
    path = f"./archives/{folder}"
    if folder in os.listdir("./archives"):
        shutil.rmtree(path)
    os.mkdir(path)
    with open(f"{path}/tmp.zip", "wb") as f:
        f.write(zip)
    with ZipFile(f"{path}/tmp.zip") as z:
        z.extractall(path)
    os.remove(f"{path}/tmp.zip")




def execute_file(data: dict) -> None:
    """
    Executes a previously stored python script.

    :param data: a dict containing "program_id" and "queue_id" entries
    :raises ValueError if the program or queue_id are invalid
    """
    raise NotImplementedError


def stop_file(data: dict) -> None:
    """
    Stops the execution of a currently running python script.

    :param data: a dict containing "program_id" and "queue_id" entries
    """
    raise NotImplementedError


def send_results(data: dict) -> None:
    """
    Sends the results of an execution to the communcation module for transmission

    :param data: a dict containing "program_id" and "queue_id" entries
    """
    raise NotImplementedError


def list_files() -> None:
    """
    Sends the currently stored python scripts to the communication module for transmission
    """
    raise NotImplementedError


def update_time(data: int) -> None:
    """
    Updates the EDU systems time.

    :param data: seconds since epoch
    """
    raise NotImplementedError
