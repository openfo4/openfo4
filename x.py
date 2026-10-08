#!/usr/bin/python3

import os
import subprocess

def main():
    cwd = os.getcwd()
    print(f"executing in {cwd}")

    env = { "RUSTFLAGS": "--cfg openfo4_embed_shaders" }

    run_command(["cargo", "build", "--release", "--bin", "openfo4_client"], env=env)

def run_command(args, cwd=None, env=None):
    cmd = " ".join(args)
    print(f"running {cmd}")

    if env is not None:
        env = {**os.environ, **env}

    handle = subprocess.Popen(args, cwd=cwd, env=env)
    status = handle.wait()
    if status != 0:
        print(f"command {cmd} exited with failure status code")
        exit(status)

if __name__ == "__main__":
    main()
