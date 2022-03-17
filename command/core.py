import logging
from typing import Any

from communication import COBC_CMD


def dispatch_command(cmd: COBC_CMD, data: Any) -> None:
    """
    This function dispatches a COBC command to the appropriate functions.

    :param cmd: Command type
    :param data: Data associated with the command
    """
    # TODO implement data preprocessing (locked by CSBI protocol)
    match cmd:
        case COBC_CMD.NOP:
            pass
        case COBC_CMD.STORE_ARCHIVE:
            store_archive(data)
        case COBC_CMD.EXECUTE_FILE:
            execute_file(data)
        case COBC_CMD.STOP_FILE:
            stop_file(data)
        case COBC_CMD.SEND_RESULTS:
            send_results(data)
        case COBC_CMD.LIST_FILES:
            list_files()
        case COBC_CMD.UPDATE_TIME:
            update_time(data)


def store_archive(data: bytes) -> None:
    """
    This function stores the received bytes as a zipped file, the unzips and copies the python script to the
    appropriate location.

    :param data: byte stream of a zip file
    :raise ValueError if the bytes are not a valid zip file
    """
    raise NotImplementedError


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
