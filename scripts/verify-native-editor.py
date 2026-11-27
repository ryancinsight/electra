"""Replay Electra's Metis HWND, then compare its Iris cursor samples with the CLI."""
import argparse
import ctypes
from ctypes import wintypes
import math
from pathlib import Path
import queue
import re
import subprocess
import tempfile
import threading
import time


def replay(editor, cli):
    user = ctypes.WinDLL("user32", use_last_error=True)
    user.FindWindowW.argtypes = [wintypes.LPCWSTR, wintypes.LPCWSTR]
    user.FindWindowW.restype = wintypes.HWND
    user.PostMessageW.argtypes = [wintypes.HWND, wintypes.UINT, wintypes.WPARAM, wintypes.LPARAM]
    user.GetWindowThreadProcessId.argtypes = [wintypes.HWND, ctypes.POINTER(wintypes.DWORD)]
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "channels.elcp"
        process = subprocess.Popen([editor, path], stdout=subprocess.PIPE, text=True)
        lines = queue.Queue()
        reader = threading.Thread(target=lambda: [lines.put(s.strip()) for s in process.stdout], daemon=True)
        reader.start()
        try:
            deadline = time.monotonic() + 30
            while True:
                hwnd = user.FindWindowW(None, "Electra schematic editor")
                owner = wintypes.DWORD()
                if hwnd and user.GetWindowThreadProcessId(hwnd, ctypes.byref(owner)) and owner.value == process.pid:
                    break
                if process.poll() is not None or time.monotonic() > deadline:
                    raise RuntimeError("editor did not create its native Metis window")
                time.sleep(0.05)

            def post(message, wparam=0, lparam=0):
                if not user.PostMessageW(hwnd, message, wparam, lparam):
                    raise ctypes.WinError(ctypes.get_last_error())

            def key(code):
                post(0x100, code, 1)
                post(0x101, code, 0xC0000001)

            def click(row):
                point = 24 | ((72 + 22 * row + 8) << 16)
                post(0x201, 1, point)
                post(0x202, 0, point)

            def command(text, expected=None):
                for character in text:
                    post(0x102, ord(character), 1)
                key(0x0D)
                result = lines.get(timeout=15)
                if expected is None:
                    expected = text.split()[0] + " complete"
                assert result.startswith(expected), (text, result, expected)
                print(text, result, flush=True)
                return result

            command("place A")
            command("place B")
            click(6)
            command("wire 1")
            click(4)
            command("wire off")
            faults = command("erc", "ERC [")
            assert "OutputConflict" in faults and "Unconnected" in faults
            command("wire 2")
            click(6)
            command("wire 3")
            assert command("erc", "ERC") == "ERC []"
            click(5)
            key(0x26)  # Focus the A load using native Up navigation.
            command("label A/TAP")
            command("bind R2k 1 2")
            command("save")
            edited = path.read_bytes()
            command("undo")
            command("save")
            assert path.read_bytes() != edited
            command("redo")
            command("save")
            assert path.read_bytes() == edited
            command("open")
            click(4)
            command("export")
            samples = [command(cmd, "A/TAP drive=") for cmd in ("simulate", "cursor 1", "cursor 2")]
            out = subprocess.check_output([cli, "dc", path.with_suffix(".cir")], text=True)
            def values(name):
                return [float(value) for value in re.findall("^" + re.escape(name) + r" = ([^ ]+)", out, re.M)]
            voltages, currents = values("V(2)"), values("I(R3)")
            assert len(voltages) == len(currents) == len(samples)
            for index, sample in enumerate(samples):
                match = re.fullmatch(r"A/TAP drive=(\S+) V voltage=(\S+) V current=(\S+) A", sample)
                assert match, sample
                drive, volts, amps = map(float, match.groups())
                assert drive == index + 1
                assert math.isclose(volts, voltages[index], rel_tol=1e-11)
                assert math.isclose(amps, currents[index], rel_tol=1e-11)
            post(0x10)
            assert process.wait(timeout=15) == 0
            print("Native Metis editor replay passed (3/3 Iris and CLI samples).")
        finally:
            if process.poll() is None:
                process.kill()
            process.wait()
            reader.join(timeout=2)
            process.stdout.close()


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("editor", type=Path)
    parser.add_argument("cli", type=Path)
    args = parser.parse_args()
    replay(args.editor.resolve(), args.cli.resolve())
