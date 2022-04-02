import unittest
import os
import filecmp

import command


class CommandHandlingTests(unittest.TestCase):
    def test_store_archive(self):
        with open("./tests/communication.zip", "rb") as f:
            data = f.read()

        command.store_archive("arch1", data)

        diff = filecmp.dircmp("./tests/test_data/", "archives/arch1")
        diff.phase4_closure()
        self.assertEqual(len(diff.diff_files), 0)
        self.assertEqual(len(diff.left_only), 0)
        self.assertEqual(len(diff.right_only), 0)


if __name__ == '__main__':
    unittest.main()
