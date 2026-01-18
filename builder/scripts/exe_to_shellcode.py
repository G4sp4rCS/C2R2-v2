#!/usr/bin/env python3
import sys, os, argparse, subprocess, shutil

def find_donut_exe():
    script_dir = os.path.dirname(os.path.abspath(__file__))
    project_root = os.path.dirname(os.path.dirname(script_dir))
    paths = [
        os.path.join(project_root, "donut_v1.1", "donut.exe"),
        os.path.join(project_root, "donut_v1.0", "donut.exe"),
        os.path.join(script_dir, "donut.exe"),
    ]
    for p in paths:
        if os.path.exists(p): return os.path.abspath(p)
    return shutil.which("donut")

def convert(input_file, output_file, arch=2, bypass=3, compress=1, entropy=3, exit_opt=1):
    donut = find_donut_exe()
    if not donut:
        print("[!] donut.exe not found in donut_v1.1/")
        return False
    print(f"[*] Using: {donut}")
    if not os.path.exists(input_file):
        print(f"[!] Input not found: {input_file}")
        return False
    input_file, output_file = os.path.abspath(input_file), os.path.abspath(output_file)
    size = os.path.getsize(input_file)
    print(f"[*] Input: {input_file} ({size:,} bytes)")
    cmd = [donut, "-i", input_file, "-o", output_file, "-a", str(arch), "-b", str(bypass), "-z", str(compress), "-e", str(entropy), "-x", str(exit_opt)]
    print(f"[*] Cmd: {' '.join(cmd)}")
    r = subprocess.run(cmd, capture_output=True, text=True, timeout=120)
    if r.stdout: print(r.stdout)
    if r.returncode != 0:
        print(f"[!] Failed: {r.stderr}")
        return False
    if os.path.exists(output_file):
        out_size = os.path.getsize(output_file)
        print(f"[+] Output: {output_file} ({out_size:,} bytes)")
        return True
    return False

if __name__ == "__main__":
    p = argparse.ArgumentParser()
    p.add_argument("input")
    p.add_argument("output")
    p.add_argument("--arch", type=int, default=2)
    p.add_argument("--bypass", type=int, default=3)
    p.add_argument("--compress", type=int, default=1)
    p.add_argument("--entropy", type=int, default=3)
    p.add_argument("--exit", type=int, default=1, dest="exit_opt")
    a = p.parse_args()
    sys.exit(0 if convert(a.input, a.output, a.arch, a.bypass, a.compress, a.entropy, a.exit_opt) else 1)
