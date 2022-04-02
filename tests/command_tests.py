import unittest
import os
import shutil
import filecmp

import command


class CommandHandlingTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.ch = command.CommandHandler()
        with open("./tests/student_program.zip", "rb") as f:
            data = f.read()
        cls.ch.store_archive("arch1", data)

    def test_store_archive(self):
        diff = filecmp.dircmp("./tests/test_data/", "archives/arch1", ignore=["results"])
        diff.phase4_closure()
        self.assertEqual(len(diff.diff_files) + len(diff.left_only) + len(diff.right_only), 0,
                         "Difference in files")

    def test_execution_normal(self):
        self.ch.execute_file("arch1", "0001")
        while self.ch.is_program_running:
            pass

        self.assertFalse(self.ch.last_program_killed)
        self.assertTrue("0001" in os.listdir("./archives/arch1/results"), "queue result folder not created")
        with open("./archives/arch1/results/0001/res.txt") as file:
            f = file.readlines()
        self.assertEqual(f, ["Some test results\n", "With multiple lines\n"], "Wrong result file contents")

    def test_execution_not_responding(self):
        self.ch.execute_file("arch1", "0002")
        while self.ch.is_program_running:
            pass
        self.assertTrue(self.ch.last_program_killed, "Endless loop not killed")

    @classmethod
    def tearDownClass(cls) -> None:
        cls.ch.student_process.kill()
        shutil.rmtree("./archives/arch1")


if __name__ == '__main__':
    unittest.main()
