import importlib
import inspect
import os
import sys

from framework import EDU_Tests

def discover_tests():
    """Automatically scans the working directory for python files with functions
    named 'test_*' and returns handles to them"""
    files = [x.strip(".py") for x in os.listdir() if x.endswith(".py")]
    modules = [importlib.import_module(f) for f in files]
    functions = [x[1] for m in modules for x in inspect.getmembers(m) if x[0].startswith("test_")]
    return functions


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == 'help':
        help_msg = """Usage: python test.py [filter]

This test runner collects all functions in python files in the working directory, that start with
'test_*'. It then tries to connect to the logic analyzer, the EDU itself and runs all tests.
If a filter is supplied, only tests with names that contain that filter are run.

Between tests, all files that were created are removed.
"""
        print(help_msg)
        exit(0)
    
    
    handle = EDU_Tests()
    handle.prepare()

    tests = discover_tests()
    if len(sys.argv) > 1:
        tests = [x for x in tests if sys.argv[1] in x.__name__]

    for t in tests:
        handle.register(t)

    exit(handle.run())