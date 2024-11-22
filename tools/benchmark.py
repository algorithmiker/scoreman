#!/usr/bin/env python3
# usually invoked via `just benchmark`
import os
import sys
import json
import shutil
import subprocess
import time
def to_canonical_revision(s:str) -> (str,str):
    return subprocess.run(["git", "rev-parse", "--short",s],capture_output=True,check=True).stdout.strip().decode("utf-8")

DEVNULL=open("/dev/null")
def run_silent(*args,**kwargs):
    subprocess.run(*args, stdout=DEVNULL, stderr=DEVNULL, **kwargs)
def main(dir_path:str,old_revision:str,new_revision:str, *args): 
    try: os.mkdir(dir_path)
    except FileExistsError: pass
    (old_revision,new_revision)=(to_canonical_revision(old_revision), to_canonical_revision(new_revision))
    for rev in (old_revision,new_revision):
        if not os.path.isfile(f"{dir_path}/gt-{rev}"):
            print(f"[I] benchmark.py: building {rev}")
            run_silent(["git", "checkout", rev])
            run_silent(["cargo", "build", "--release"], check=1)
            shutil.copy("target/release/guitar_tab", f"{dir_path}/gt-{rev}")
    
    run_silent(["git", "checkout", "master"])
    print("[I]: benchmark.py: have everything, sleeping 0.5s to avoid interference")
    time.sleep(0.5)
    args_str=" ".join(args[0])
    subprocess.run(["hyperfine", f"{dir_path}/gt-{old_revision} {args_str}", f"{dir_path}/gt-{new_revision} {args_str}", "--warmup", "30", "-N"])
    print(f"[I]: benchmark.py: {old_revision=} {new_revision=}")

if __name__=="__main__":
    if len(sys.argv) < 3: 
        print("USAGE: benchmark <benchmark_dir> <old_rev?> <new_rev?> -- <guitar_tab_arguments*>")
        sys.exit(1)
    sep_before=False
    prev=None
    new=None
    rest = []
    for arg in sys.argv[2:]:
        if arg=='--': sep_before=True
        elif prev==None and not sep_before: prev=arg
        elif new==None and not sep_before: new=arg
        else: rest.append(arg)
    if prev==None: prev="HEAD~1"
    if new == None: new="HEAD"
    main(sys.argv[1], prev,new, rest)
