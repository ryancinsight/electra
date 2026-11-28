"""Replay held-out placements on Linux with separate-process wall time and peak RSS."""
import argparse
import csv
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time

HEADER = "board sha256 policy seed status connected nets overused opens shorts vias length_mm kicad_errors kicad_unconnected elapsed_s peak_kib".split()

def measure(cmd, output, log, budget):
    start = time.monotonic()
    with output.open("w") as dst, log.open("w") as err:
        child = subprocess.Popen(cmd, stdout=dst, stderr=err)
        timeout = False
        while True:
            pid,status,usage = os.wait4(child.pid, os.WNOHANG)
            if pid: break
            if time.monotonic()-start >= budget:
                child.kill()
                pid,status,usage = os.wait4(child.pid, 0)
                timeout = True
                break
            time.sleep(.005)
        child.returncode = os.waitstatus_to_exitcode(status)
    return ("timeout" if timeout else "ok" if child.returncode==0 else "error",
            round(time.monotonic()-start,3), usage.ru_maxrss)

def run(exe, sources, budget):
    writer=csv.writer(sys.stdout, lineterminator="\n")
    writer.writerow(HEADER)
    with tempfile.TemporaryDirectory() as directory:
        d=Path(directory)
        for source in sources:
            digest=hashlib.sha256(source.read_bytes()).hexdigest()
            data=d/"input"
            with data.open("w") as dest:
                subprocess.run([sys.executable,Path(__file__).with_name("import.py"),source],stdout=dest,check=True)
            for seed in (71,117):
                for policy in ("driver","research"):
                    board=d/"trial.kicad_pcb"
                    out,log=d/"metrics",d/"stderr"
                    status,sec,rss=measure([exe,data,policy,str(seed),board],out,log,budget)
                    metrics=[""]*9
                    if status=="ok":
                        drc=d/"drc.json"
                        subprocess.run(["kicad-cli","pcb","drc","--format","json","--severity-error","-o",drc,board],stdout=subprocess.DEVNULL,check=True)
                        report=json.loads(drc.read_text())
                        metrics=out.read_text().strip().split(",")+[
                            len(report["violations"]), len(report["unconnected_items"])]
                    else:
                        print(log.read_text(),file=sys.stderr)
                    writer.writerow([source.stem,digest,policy,seed,status,*metrics,sec,rss])
                    sys.stdout.flush()

if __name__=="__main__":
    p=argparse.ArgumentParser()
    p.add_argument("binary",type=Path)
    p.add_argument("sources",nargs="+",type=Path)
    p.add_argument("--seconds",type=float,default=60)
    args=p.parse_args()
    run(args.binary.resolve(), args.sources,args.seconds)
