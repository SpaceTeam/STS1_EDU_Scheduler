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
        help_msg = """Usage: python test.py [all|quick|full]

This test runner collects all functions in python files in the working directory, that start with
'test_*'. It then tries to connect to the logic analyzer and runs either all tests, all tests
containing 'quick' in their name or all tests not containing 'quick' (full).

Full tests imply a complete reset of the raspi (power off), while quick tests only clean up
temporary files and restart the scheduler. 
"""
        print(help_msg)
        exit(0)
    
    
    handle = EDU_Tests()
    handle.prepare()

    tests = discover_tests()
    for t in tests:
        if "quick" in t.__name__:
            handle.register(t, 'quick')
        else:
            handle.register(t, 'full reset')

    if len(sys.argv) < 2:
        run = 'all'
    elif sys.argv[1] in ["all", "quick", "full"]:
        run = sys.argv[1]
    else:
        print("Call the testing framework with either 'all', 'quick' or 'full' as argument")
        exit(-1)

    exit(handle.run(run))