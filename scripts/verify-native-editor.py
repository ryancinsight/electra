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
            command("bus taps 4 2")
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
            command("save")
            assert path.read_bytes() == edited
            click(4)
            assert command("inspect", "sheet") == "sheet A part 3 pin 1 model R2k [1, 2]"
            assert command("lanes taps", "lanes") == "lanes 4 2"
            assert command("junction 2", "pins") == "pins 2.2 3.1"
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
            fixture = Path(__file__).resolve().parents[1] / "examples/field/two-rails.rail"
            path.with_suffix(".rail").write_bytes(fixture.read_bytes())
            for mode, zoom in [("drop", 1), ("thermal", 2)]:
                command(f"field {mode} {zoom}", f"field {mode} zoom {zoom}")
                point = (380 + 36 * zoom) | ((320 + 12 * zoom) << 16)
                post(0x201, 1, point)
                post(0x202, 0, point)
                sample = lines.get(timeout=15)
                match = re.fullmatch(r"net 7 x=(\S+) mm y=(\S+) mm drop=(\S+) V rise=(\S+) K", sample)
                assert match, sample
                x, y, drop, rise = map(float, match.groups())
                cold = 1.724e-8 * 0.1 / (0.0005 * 35e-6)
                expected = 0.
                for _ in range(80):
                    expected = 60 * 4 * cold * (1 + 0.00393 * expected)
                assert math.isclose(x, 13.) and math.isclose(y, 21.)
                assert math.isclose(rise, expected) and math.isclose(drop, expected / 120.)
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
